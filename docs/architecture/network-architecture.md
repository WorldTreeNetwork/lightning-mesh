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
2. Router-1 kernel: 10.42.2.0/24 is not local, route table says: dev mj-tun0
3. Daemon reads packet from mj-tun0 → encapsulates → sends via Iroh to Router-5
4. Router-5 daemon: decapsulates → writes to its mj-tun0 → kernel delivers to 10.42.2.30
5. Return traffic follows the same path in reverse
```

### Linux routing setup

Each router runs a TUN device managed by the mjolnir-mesh daemon. Routes are installed as remote
subnets are discovered via the CRDT:

```bash
# Iroh tunnel interface created by the daemon at startup
ip link add mj-tun0 type tun
ip addr add 10.42.0.1/32 dev mj-tun0
ip link set mj-tun0 up

# Route to Site B's subnet, installed when Router-5 appears in CRDT
ip route add 10.42.2.0/24 dev mj-tun0

# iptables: only forward traffic between known mesh subnets
iptables -A FORWARD -i mj-tun0 -o br-lan -m set --match-set mesh-subnets dst -j ACCEPT
iptables -A FORWARD -i br-lan -o mj-tun0 -m set --match-set mesh-subnets dst -j ACCEPT
iptables -A FORWARD -i mj-tun0 -j DROP
```

The daemon owns the read-side of `mj-tun0`. Packets read from the TUN are matched against the
route table CRDT to select the correct Iroh connection, then written into the QUIC stream.
Incoming packets from Iroh are written back to `mj-tun0` for kernel delivery.

---

## Route Table CRDT

Remote subnets are announced via the shared CRDT document:

```
/routes/10.42.1.0_24  → { node_id: "router1_nodeid", site: "site-a", expires: <timestamp> }
/routes/10.42.2.0_24  → { node_id: "router5_nodeid", site: "site-b", expires: <timestamp> }
```

When a new route appears in the CRDT, the daemon:
1. Opens (or reuses) an Iroh connection to the announcing `node_id`
2. Installs the Linux route: `ip route add {subnet} dev mj-tun0`
3. Updates the internal forwarding table with the Iroh stream for that subnet

When a route disappears (router offline or TTL expired), the daemon:
1. Removes the Linux route: `ip route del {subnet} dev mj-tun0`
2. Closes the Iroh connection if no remaining routes reference it
3. DNS entries for that subnet's devices are retained in the CRDT but the hosts may be unreachable

### Stale route prevention

Routes go stale when a remote router dies without a clean disconnect. The daemon defends against
this through three overlapping mechanisms:

- **Iroh connection state**: The QUIC transport detects peer loss within seconds. When the
  connection drops, the daemon removes all routes associated with that `node_id` immediately.
- **Heartbeat gossip**: Routers publish a liveness announcement every 30 seconds. No heartbeat
  for 90 seconds causes the daemon to mark routes stale and remove them from the Linux table.
- **Route TTL**: Each CRDT route entry carries an expiry timestamp. The announcing router refreshes
  the TTL on each heartbeat. Expired entries are removed by any daemon that notices them, a
  last-write-wins CRDT tombstone handles concurrent deletions cleanly.
- **Daemon restart**: On startup the daemon rebuilds the route table from the current CRDT state
  filtered by active Iroh connections, discarding any routes whose gateway is unreachable.

---

## Subnet Allocation for Remote Sites

When a router determines it is starting a new isolated site (no local peers detected within the
detection window), it claims a /24 from the mesh address space:

1. Read the CRDT `/routes/` prefix to enumerate already-claimed subnets
2. Derive a preferred /24 from a deterministic hash of the router's Iroh NodeId — this reduces
   collisions without requiring coordination
3. If the preferred /24 is already claimed, increment until a free one is found
4. Write to CRDT: `/routes/{subnet} → { node_id, site, expires }`
5. Configure dnsmasq with that range and begin issuing leases
6. If another router later joins the same physical site, it detects the local peer (see below),
   abandons its own subnet claim, and joins the existing /24 in Mode 1 instead

The two-phase approach (derive then check) is optimistic: hash-based derivation makes collisions
rare, and the CRDT resolves the uncommon case where two routers happen to prefer the same /24.

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
4b. [Future] Offer the same IP (10.42.1.50) — Router-5 updates the CRDT route to claim
    that /32 host route, Router-1 withdraws it. TCP sessions survive.
```

Option (a) is the MVP behavior: simpler implementation, no host-route management, but TCP
sessions break on roam. Option (b) enables seamless cross-site roaming at the cost of per-device
route table churn and is deferred to a later milestone.

---

## Security

Traffic between sites is secured at the transport layer by Iroh:

- All Iroh connections use QUIC with TLS 1.3 — encryption is mandatory and cannot be disabled
- Router identity is bound to the Iroh NodeId (Ed25519 keypair); only nodes with NodeIds
  listed in the mesh membership CRDT can join and exchange routes
- IP forwarding on each router is restricted to known mesh subnets via iptables rules — arbitrary
  external traffic cannot be injected through the tunnel
- No open relay: the Iroh relay servers are used only for NAT traversal handshake, not for
  sustained packet forwarding between routers

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

- CRDT design and hostsfile synchronization: `docs/architecture/dhcp-crdt.md`
- dnsmasq configuration and lease management: `docs/architecture/dnsmasq-integration.md`
- Top-level mesh coordination overview: `docs/mesh-network-coordination.md`