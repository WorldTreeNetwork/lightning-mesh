# Network Architecture

## Overview

mjolnir-mesh creates a decentralized mesh network across OpenWrt routers. Routers that share a
physical network (same L2 broadcast domain) bridge into a unified subnet, presenting a single flat
network to devices. Routers at different physical sites connect via Iroh QUIC tunnels and route IP
traffic between their local subnets. Both modes coexist: a DWEB event might have 10 co-located
routers sharing one /24 plus 3 remote participants tunneled in from separate locations.

```
Site A (local L2)                          Site B (remote)
 ┌──────────────────────────┐               ┌──────────────────┐
 │  Router-1  Router-2  ... │               │     Router-5     │
 │  10.42.1.0/24            │◄──Iroh QUIC──►│  10.42.2.0/24   │
 │  (shared subnet)         │               │                  │
 └──────────────────────────┘               └──────────────────┘
```

---

## Two Modes of Interconnection

### Mode 1: Local (Same L2)

When routers can reach each other directly — same ethernet switch, same WiFi backhaul, or wired
together at a single venue — they operate in local mode.

- Routers detect each other via mDNS (`_mjolnir-mesh._tcp.local`) or Iroh connection latency
  below the local threshold (~5ms round-trip)
- All local routers share a single subnet (e.g., `10.42.1.0/24`)
- Each router runs dnsmasq covering the full shared range; the CRDT hostsfile prevents IP
  conflicts by distributing MAC-to-IP bindings across all nodes
- Devices see one flat broadcast domain and can reach any device on any AP without routing
- mDNS, Bonjour, and AirPlay work natively because they remain on the same broadcast segment
- Roaming between APs is seamless: the device keeps the same IP, and dnsmasq on the new AP
  already has the MAC reservation from the CRDT

### Mode 2: Remote (Via Iroh)

When routers are at different physical locations, they connect through Iroh QUIC tunnels.

- Each site has its own subnet (e.g., Site A: `10.42.1.0/24`, Site B: `10.42.2.0/24`)
- Iroh provides NAT traversal, encryption, and relay fallback — no port forwarding required
- The mjolnir-mesh daemon manages a TUN interface on each router and encapsulates IP packets
  into the Iroh QUIC stream for delivery to the remote site
- DNS is synced via CRDT so any device can resolve hostnames registered at any site
- mDNS is forwarded across sites via avahi-daemon in reflector mode

---

## Cross-Site Routing

### Packet flow

```
Alice (10.42.1.50, Router-1 at Site A) → Bob's server (10.42.2.30, Router-5 at Site B)

1. Alice sends to 10.42.2.30 (or bob-server.mesh resolved via DNS)
2. Router-1 kernel: 10.42.2.0/24 is in babeld-installed route table: dev mj-peer-<router5_id>
3. Daemon reads packet from mj-peer-<router5_id> → encapsulates → sends via Iroh to Router-5
4. Router-5 daemon: decapsulates → writes to its mj-peer-<router1_id> → kernel delivers to 10.42.2.30
5. Return traffic follows the same path in reverse
```

### Linux routing setup

Each router exposes one TUN interface **per active Iroh peer**, managed by the mjolnir-mesh daemon. Babel (`babeld`) runs on each router, peers over those TUN interfaces, and installs/withdraws Linux routes as remote subnets become reachable.

```bash
# Per-peer Iroh tunnel interface, created on Iroh connect
ip link add mj-peer-aabbccdd type tun
ip addr add 10.255.0.1/31 dev mj-peer-aabbccdd  # link-local /31 from reserved 10.255.0.0/16
ip link set mj-peer-aabbccdd up

# babeld peers on this interface and learns 10.42.2.0/24 from Router-5
# babeld installs the route directly:
# ip route add 10.42.2.0/24 dev mj-peer-aabbccdd  ← done by babeld via netlink

# iptables: only forward traffic between known mesh subnets
iptables -A FORWARD -i mj-peer-+ -o br-lan -m set --match-set mesh-subnets dst -j ACCEPT
iptables -A FORWARD -i br-lan -o mj-peer-+ -m set --match-set mesh-subnets dst -j ACCEPT
iptables -A FORWARD -i mj-peer-+ -j DROP
```

The daemon owns the read-side of each `mj-peer-<id>` TUN. Packets read from a TUN are encapsulated and sent into the corresponding Iroh QUIC stream; incoming packets from Iroh are written back to the matching TUN for kernel delivery. Forwarding decisions are made by the kernel from babeld-installed routes — the daemon does not maintain its own forwarding table.

See [babel-routing.md](babel-routing.md) for the full Babel integration spec, including babeld config, failure modes, and the rationale for delegating routing to a battle-tested protocol.

---

## Subnet Claim Coordination (CRDT)

The CRDT no longer holds a routing table. It holds a **subnet ownership ledger** used only to prevent two routers from claiming the same /24 at first boot:

```
/subnets/10.42.1.0_24  → { owner_node_id: "router1_nodeid", site_name: "site-a", claimed_at: <hlc> }
/subnets/10.42.2.0_24  → { owner_node_id: "router5_nodeid", site_name: "site-b", claimed_at: <hlc> }
```

When a router claims a subnet, it writes one entry and reconfigures babeld to redistribute that subnet. Babel handles announcement, propagation, and route installation to all peers. The CRDT is *not* consulted for forwarding decisions.

Conflicts on `/subnets/` (two routers claim the same /24) resolve via HLC first-writer-wins, same rule as IP-lease conflicts. The loser picks the next free /24 and rewrites its claim.

When a router goes offline:
- Iroh disconnect tears down the per-peer TUN
- Babel marks the route unreachable within its hello interval and withdraws it
- The `/subnets/` entry persists (the subnet is still *claimed*, just not reachable). On the owner's reboot, Babel re-announces; on a graceful permanent departure, the daemon tombstones the entry.

No heartbeat gossip, no route-TTL refresh, no daemon-side stale-route reaping. Babel handles all of that.

---

## Subnet Allocation for Remote Sites

When a router determines it is starting a new isolated site (no local peers detected within the
detection window), it claims a subnet from the mesh address space. The operator picks the subnet
**size**; the allocator picks the **slot**. Larger requests (smaller prefix lengths) are for
larger sites — a /24 fits 254 devices, /22 fits ~1 000, /20 fits ~4 000, /16 fits ~65 000.

The size is configurable per router. The expected UX is a TUI selector where arrow keys step
the prefix one bump at a time (`/24 ↔ /23 ↔ /22 ↔ …`), with a label showing the resulting IP
count. Backed by `mjolnir_mesh::alloc::{pick_subnet, bump_larger_subnet, bump_smaller_subnet,
usable_hosts}`.

1. Read the CRDT `/subnets/` prefix to enumerate already-claimed subnets (at any size).
2. Operator chooses a target prefix length (default /24 if unconfigured).
3. Compute the preferred slot from a deterministic hash of the router's Iroh NodeId. Hash bytes
   modulo `2^(target_prefix - base_prefix)` index into the candidate slots at the chosen size.
4. Walk slots from the preferred index. Reject any candidate that **overlaps** an existing claim
   of any size — a candidate /22 is rejected if it contains a claimed /24, and a candidate /24 is
   rejected if a containing /22 is already claimed. (CIDR blocks form a tree: overlap reduces to
   `a.contains(b) || b.contains(a)`.)
5. Write to CRDT: `/subnets/{cidr} → { owner_node_id, site_name, claimed_at }`.
6. Configure dnsmasq with that range and begin issuing leases.
7. Add a `redistribute ip {cidr} ge {prefix} le {prefix} allow` line to babeld config; SIGHUP babeld.
8. If another router later joins the same physical site, it detects the local peer (see below),
   abandons its own subnet claim, and joins the existing subnet in Mode 1 instead.

The two-phase approach (derive then check) is optimistic: hash-based derivation makes collisions
rare, and the CRDT resolves the uncommon case where two routers happen to prefer the same slot.

### Claim Cooldown

A router that determines it needs a new subnet waits 10 seconds before writing the claim to the CRDT. This aligns with the local peer detection window — if a local peer is found during the cooldown, the router abandons the claim and joins the existing subnet instead.

If two routers at different sites claim overlapping subnets (rare — hash derivation across the
slot count at the chosen prefix makes collision probability `~1/N` where N is the slot count):
- FWW resolves: lower HLC wins the `/subnets/` entry
- The loser has zero or very few devices (it just started) — it picks the next free slot at its
  configured prefix length, rewrites its claim, and updates babeld redistribute config
- If the loser has already assigned IPs from the contested range, those devices are deauthed and
  re-DHCP on the new range
- If no free slot exists at the loser's chosen prefix length, the allocator returns `None` and
  the daemon must escalate (operator widens the mesh space, picks a smaller subnet size, or fails
  loud — the library does not silently shrink the request)

### Late Local Peer Discovery

If a router claims a subnet and then discovers (via delayed mDNS or gossip) that its site already has one:
1. Relinquish its subnet claim (tombstone `/subnets/{cidr}` in CRDT)
2. Remove the corresponding `redistribute` line from babeld config; SIGHUP babeld
3. Reconfigure dnsmasq to the existing site subnet
4. Deauth any devices already assigned IPs from the abandoned range
5. Those devices reconnect and get IPs from the correct subnet

---

## Local Peer Detection

A router entering a venue needs to determine whether it is joining an existing local cluster or
starting a new remote site. Detection uses multiple signals in parallel:

| Method | Signal | Reliability |
|---|---|---|
| Iroh connection latency | Round-trip < 5ms → same LAN | High for wired, variable for WiFi |
| mDNS | `_mjolnir-mesh._tcp.local` announcement | Requires mDNS-capable network |
| UDP broadcast probe | Send to 255.255.255.255 on mesh port, wait for reply | Works on flat L2 |
| Manual config | `--local-peers=nodeA,nodeB` | Authoritative override |

Detection uses a 10-second window after startup. If any signal identifies a local peer, Mode 1
applies. If no local peers are found within the window, the router proceeds as a new remote site.

When a local peer is confirmed:
- The router does not claim a new subnet
- It bridges into the peer's existing L2 segment
- dnsmasq is configured with the peer's subnet range; the CRDT hostsfile ensures no IP conflicts
- The router announces itself to the local cluster via the shared CRDT

---

## SSID and VLAN Guidance

**Recommended:** All routers use the same SSID and the same VLAN (one broadcast domain). This gives the best experience: seamless roaming, single subnet, native mDNS/Bonjour.

**Different SSIDs, same VLAN:** Works identically. DHCP operates at L2; the SSID is irrelevant once frames are bridged to the shared VLAN. Devices may not roam automatically between different SSIDs (depends on client behavior), but the network coordination is unaffected.

**Different VLANs (even if co-located):** Each VLAN is a separate L2 domain. Routers on different VLANs cannot exchange DHCP broadcasts and are treated as Mode 2 (remote sites) with separate subnets and Iroh tunnel routing, regardless of physical proximity.

**Multi-VLAN on one router:** Future extension. Would require per-VLAN dnsmasq instances and per-VLAN hostsfile management. Not in scope for MVP.

---

## Address Space

The default mesh address space is `10.42.0.0/16`, providing:
- 65,534 usable host addresses
- Up to 256 independent /24 subnets (one per remote site)
- 254 devices per /24 at a single site

For deployments that expect more devices at a single venue, the site subnet can be widened:
- `/23` — 510 devices
- `/20` — 4,094 devices
- `/16` — 65,534 devices (useful for large events sharing one physical network)

For federations requiring more than 256 sites, the mesh address space can be expanded to
`10.0.0.0/8` at deployment time, supporting up to 65,536 /24 subnets across 16 million addresses.
Address space configuration is set in the CRDT root document and read by all nodes on join.

---

## Roaming Across Sites

A device physically moving from Site A to Site B:

```
1. Device disconnects from Site A's WiFi (Router-1)
2. Device connects to Site B's WiFi (Router-5)
3. Router-5's dnsmasq checks the CRDT hostsfile
4a. [MVP] Device's MAC has a binding at 10.42.1.x — offer a new IP from Site B's range (10.42.2.x)
    DNS entry updated via CRDT. TCP sessions break; new sessions work immediately.
4b. [Future] Offer the same IP (10.42.1.50) — Router-5 redistributes a /32 host route
    via babeld, Router-1 withdraws its /32 advertisement. TCP sessions survive.
```

Option (a) is the MVP behavior: simpler implementation, no host-route management, but TCP
sessions break on roam. Option (b) enables seamless cross-site roaming by letting Babel carry per-device /32 routes — natively supported but deferred to a later milestone for operational simplicity.

---

## Security

Traffic between sites is secured at the transport layer by Iroh:

- All Iroh connections use QUIC with TLS 1.3 — encryption is mandatory and cannot be disabled
- Router identity is bound to the Iroh NodeId (Ed25519 keypair); membership enforcement is
  planned, not yet implemented (see Future Work below)
- IP forwarding on each router is restricted to known mesh subnets via iptables rules — arbitrary
  external traffic cannot be injected through the tunnel
- No open relay: the Iroh relay servers are used only for NAT traversal handshake, not for
  sustained packet forwarding between routers

### End-to-end vs per-hop confidentiality

Where Iroh carries the **data plane** (cross-site tunnels today; optionally same-site too — see
the single-overlay-TUN work, `buw`), confidentiality is **end-to-end between the two router
daemons**: every packet rides inside a QUIC / TLS 1.3 connection, so no intermediate node — not
even another mesh router relaying the datagrams — can read it.

Contrast the radio backhaul. 802.11s with SAE encrypts each **radio hop** independently: a
multi-hop frame is decrypted and re-encrypted at every forwarding node, so an intermediate mesh
router *does* see plaintext. That is fine for a single trusted hop, but it is **not** end-to-end.

So: cross-site traffic over the Iroh overlay is confidential even from the routers relaying it;
native 802.11s same-site traffic is only hop-by-hop confidential. Routing same-site over the
overlay as well (the `buw` "U1" option) extends end-to-end confidentiality to every hop, at the
cost of QUIC encapsulation on the local radio link. This property only holds where Iroh is the
data plane — it is a reason to *prefer* the overlay where confidentiality from intermediate
nodes matters.

### Future Work: Membership Control

**Current gap:** Any Iroh node that knows the gossip topic can join the mesh and inject data. There is no membership enforcement.

**Phase 1 (MVP):** Pre-shared key (PSK) configured on each router. The gossip topic is derived from `blake3(b"mjolnir/mesh/" || psk)` instead of a static name, preventing unauthorized joining. Simple but key rotation requires touching every router.

**Phase 2:** Membership CRDT (`/members/{node_id}`) with signed enrollment invitations. Any existing member can invite a new node by signing its public key. Peers validate membership before accepting gossip messages. Revocation via CRDT tombstone.

---

## Relationship to Iroh and IPv4

The two layers have complementary roles:

**Iroh provides:**
- Encrypted QUIC transport between routers (the tunnel substrate)
- Global router identity via NodeId
- Peer discovery (n0 discovery service, gossip-based mesh discovery)
- NAT traversal (hole-punching with relay fallback)
- CRDT replication via gossip

**IPv4 provides:**
- Standard LAN connectivity for consumer devices and applications
- Routing abstraction: cross-site IP reachability without application changes
- DNS: familiar hostname resolution for devices on the mesh

Neither layer is aware of the other's internals. IPv4 packets are opaque payloads from Iroh's
perspective; the mjolnir-mesh daemon is the bridge that reads from the TUN device and writes to
the Iroh stream (and vice versa). This clean separation means Iroh can be upgraded or replaced
without touching the routing logic, and the IPv4 topology can be reconfigured independently of
the underlying transport.

---

## References

- CRDT design and hostsfile synchronization: `dhcp-crdt.md`
- dnsmasq configuration and lease management: `dnsmasq-integration.md`
- Babel routing integration: `babel-routing.md`
- Top-level mesh coordination overview: `mesh-network-coordination.md`