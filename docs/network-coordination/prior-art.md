# Prior Art and External Comparison

**Status:** Reference | **Date:** 2026-05-18

How mjolnir-mesh compares to existing approaches for mesh DHCP, routing, and service discovery. Written to (a) make our design choices auditable against state-of-the-art, (b) flag where we diverge from convention deliberately, and (c) preempt the "why didn't you just use X" question.

---

## 1. The Three Reference Points

### 1.1 CeroWrt + AHCP + Babel

[CeroWrt](https://www.bufferbloat.net/projects/cerowrt/wiki/Mesh/) is the canonical OpenWrt-based mesh distro from the bufferbloat.net group. Its stack:

| Layer | Choice |
|---|---|
| Address allocation | **AHCP** (Ad-Hoc Configuration Protocol) — single designated server per network, hands out IPs + DNS + NTP from configured ranges |
| Subnet partitioning | Manual: "unique subnet number per router" — operators assign ranges out-of-band |
| Routing | **Babel** (default), with OLSR / batman / Quagga BGP as alternatives |
| DNS | Distributed via AHCP-configured resolvers; no mesh-wide directory |
| Service discovery | None at the routing layer; standard DNS + per-host mDNS |
| IPv4 vs IPv6 | Dual-stack; AHCP can hand out either or both |

**Key insight:** CeroWrt separates *configuration* (AHCP) from *routing* (Babel). They are independent protocols with different roles. Our `dhcp-crdt` ↔ `babel-routing` split mirrors this.

### 1.2 AHCP (standalone)

[AHCP](https://www.irif.fr/~jch/software/ahcp/) by Juliusz Chroboczek, designed for ad-hoc networks where DHCP cannot reach every L2 broadcast domain. From the spec:

> "AHCP is an autoconfiguration protocol for IPv6 and dual-stack IPv6/IPv4 networks designed to be used in place of router discovery and DHCP on networks where it is difficult or impossible to configure a server within every link-layer broadcast domain."

| Property | AHCP |
|---|---|
| Server model | One or a few designated servers per network |
| Address allocation | Pulls from a pre-configured range; servers do not coordinate, operators ensure non-overlap |
| Routing | Explicitly **not** configured by AHCP — "designed to be run together with a routing protocol such as Babel or OLSR" |
| Replacement | Author plans to deprecate in favor of **HNCP** (Homenet) |
| IP version bias | IPv6-first, IPv4 supported |

**Key insight:** AHCP still has a writer asymmetry — one (or a small number of) servers hands out addresses. It does not solve the "every router can independently assign IPs without conflict" problem because operators ensure non-overlapping ranges out-of-band.

### 1.3 OpenWISP

[OpenWISP](https://openwisp.io/docs/24.11/tutorials/mesh.html) is a centrally-managed network configuration platform for fleets of OpenWrt devices.

| Layer | Choice |
|---|---|
| Address allocation | Punt: assumes one external LAN DHCP server, bridges mesh into that LAN |
| Routing | "Out of scope of this tutorial" |
| DNS | Not addressed |
| Service discovery | Not addressed |
| IPv4 vs IPv6 | Disables DHCPv6 explicitly, no rationale given |

**Key insight:** OpenWISP is centrally orchestrated — it solves "how do I configure 100 OpenWrt routers consistently" by having a control plane, not by making the routers themselves coordinate. Their mesh tutorial is fundamentally a different problem than ours.

---

## 2. Comparison Matrix

| Concern | CeroWrt + AHCP | OpenWISP | mjolnir-mesh |
|---|---|---|---|
| **DHCP writers** | 1 server per network | 1 external LAN server | **N** — every router |
| **Conflict prevention** | Manual range partitioning | Single writer eliminates conflicts | **Gossiped reservations hostsfile + deauth-on-conflict** |
| **Liveness model** | Leader assumed up | Leader assumed up | **Symmetric — any router can die** |
| **Routing** | Babel | None / external | **Babel** (after this revision) |
| **Service discovery** | None at mesh layer | None | **CRDT-replicated `/services/` directory** |
| **DNS scope** | Per-resolver | LAN-scoped | **Mesh-wide via CRDT** |
| **State sync** | None (config is static) | Push from controller | **iroh-gossip + anti-entropy** |
| **Identity** | IP + manual config | OpenWISP UUID | **Iroh NodeId (Ed25519)** |
| **NAT / cross-site** | Out of scope | Out of scope | **Iroh QUIC tunnels with hole-punching** |
| **IP version** | Dual-stack | IPv4 only (DHCPv6 off) | **IPv4 today, v6-ready data model** |

---

## 3. Where We Diverge — and Why

### 3.1 Symmetric multi-writer DHCP

**Diverges from:** all three references — every prior art has exactly one DHCP writer (CeroWrt one AHCP server, OpenWISP one LAN server, no one does multi-writer).

**Why we diverge:** target deployment is DWEB events with 10+ co-located routers and aggressive device roaming. A single-writer model creates a single point of failure (lose the leader, lose new leases) and complicates roaming (every roam is a renew, and the leader must know about it). Symmetric multi-writer with CRDT reconciliation eliminates the leader and makes roaming a CRDT update, not a leader-mediated handoff.

**Cost:** the ~100ms conflict window and the deauth-on-conflict path. Both are bounded: conflicts are rare, deauth recovery is ~2s, and prior art tolerates worse failure modes (lost leader = no new leases until manual intervention).

### 3.2 Mesh-wide service directory

**Diverges from:** prior art doesn't address this at all. Standard practice is per-host mDNS + avahi reflectors for cross-router visibility.

**Why we diverge:** mDNS reflectors are operationally fragile — loops, name collisions on reconnect, multicast storms in larger meshes. A gossiped `/services/{name}` directory sidesteps the entire reflector mess and is naturally tied to device lease lifecycle (via `host_mac`), so service cleanup is automatic.

**Cost:** clients still need to query the mesh DNS resolver (not mDNS directly) to find these services. Standard mDNS clients on devices won't see CRDT-only services unless we also forward them as mDNS announcements. (Future work.)

### 3.3 IPv4-only

**Diverges from:** AHCP and CeroWrt are dual-stack with an IPv6 lean; Homenet/HNCP (the planned AHCP successor) is IPv6-first.

**Why we diverge:** the project's UX premise — *"my laptop is `10.42.1.50`, my printer is `printer.mesh`"* — is built on memorable addresses. Home/office IoT devices still have buggy IPv6 support in 2026. The novel work here is the symmetric coordination layer, not the protocol stack; running v4-only lets us focus that work without IPv6 corner-case maintenance.

**Cost:** out of step with the mesh-routing research community. Mitigated by making the data model IP-version-agnostic (`LeaseEntry.ip` is `std::net::IpAddr`, not `Ipv4Addr`) so v6 is additive when added.

### 3.4 CRDT for coordination, not routing

**Diverges from:** an earlier draft of mjolnir-mesh itself, which tried to use the CRDT as a routing table (`/routes/{subnet}` with `via_node_id` and TTLs).

**Why we converged with prior art:** Babel exists, ships in OpenWrt, has 15+ years of validation, handles loop-free reconvergence in seconds, and has explicit support for non-multicast (tunnel) interfaces. Reinventing this is a multi-year project for no user-visible win.

**See:** [babel-routing.md](babel-routing.md) for the full integration.

---

## 4. What We Match

### 4.1 Babel for cross-site routing
Adopted from CeroWrt's lead. Same protocol, similar role — routing over wireless/tunneled links between sites.

### 4.2 Separation of configuration and routing
Mirrors AHCP's design: address management and routing are independent protocols. Our `dhcp-crdt` ↔ `babel-routing` split is the same pattern with different mechanics.

### 4.3 dnsmasq as the DHCP/DNS frontend
Standard OpenWrt choice. We don't reinvent the local-protocol layer — we feed dnsmasq through its standard `dhcp-hostsfile` and `addn-hosts` integration points and signal SIGHUP for reloads.

---

## 5. What Doesn't Exist Anywhere Yet

The contribution of mjolnir-mesh, beyond the integration work:

1. **Symmetric multi-writer DHCP across a mesh with CRDT-driven conflict resolution.** Nobody does this — prior art either has a single writer or partitions the IP pool. Our deauth-on-conflict path is novel enough to be worth a paper if anyone cared to write one.

2. **CRDT-replicated cross-mesh service directory bound to device lease lifecycle.** Existing mesh service discovery is mDNS-reflector-based and fragile. Our gossiped `/services/` namespace with `host_mac` tying is a cleaner model.

3. **Iroh NodeId as the cross-NAT identity substrate.** Existing mesh protocols assume L2 adjacency or pre-configured IP peering. Iroh gives us a stable cryptographic identity that traverses NAT, which is what makes "your home router and the conference WiFi join the same mesh" feasible without VPN-server provisioning.

The first two are explicit design contributions; the third is leverage from the Iroh dependency.

---

## 6. Open Reading

- **Babel RFC 8966** (the protocol).
- **HNCP RFC 7788** (Homenet — the IPv6-first AHCP successor; worth understanding before our v6 work).
- **CeroWrt Mesh wiki** (above).
- **Freifunk** — German community-mesh project; uses batman-adv. We're explicitly not using batman (L2 protocol, less observable than L3).
- **B.A.T.M.A.N.-V** — modernized batman with Babel-like properties; we still chose Babel for the broader OpenWrt tooling support.

---

## 7. References

- [babel-routing.md](babel-routing.md) — our Babel integration spec
- [dhcp-crdt.md](dhcp-crdt.md) — CRDT data model
- [network-architecture.md](network-architecture.md) — cross-site topology
- [mesh-network-coordination.md](mesh-network-coordination.md) — overall architecture
