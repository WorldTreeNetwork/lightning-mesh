# P2P Resilience: Centralization Analysis & Plan

mjolnir-mesh is an Iroh-based mesh VPN with distributed network coordination: DHCP,
DNS, routing, and service discovery are all CRDT-synchronized across mesh nodes. This
doc analyzes where centralization still exists, what the failure modes are, and what
the roadmap looks like for improving resilience.

See [mesh-network-coordination.md](mesh-network-coordination.md) for the full
architecture and [mesh-network-crdt.md](mesh-network-crdt.md) for the CRDT data model.

---

## Centralization Analysis

### Ticket-based joining (ticket.rs)

Tickets are the bootstrap mechanism for new nodes entering the mesh. A ticket embeds
one or more `NodeAddr` values (multi-address support is already implemented) — Iroh
node IDs paired with their known addresses. The joiner tries each address until one
succeeds.

```rust
pub struct MeshTicket {
    pub name: String,
    pub addrs: Vec<NodeAddr>,   // multi-address: any live peer works
    pub topic_id: [u8; 32],
}
```

**Remaining centralization:** The ticket must be obtained out-of-band (shared link,
QR code, PSK-derived topic). The topic_id is currently derived deterministically from
the room name; a PSK-based derivation would prevent uninvited nodes from computing the
topic independently.

**Already good:** Multi-address tickets mean no single peer is a required bootstrap
point. Any peer in the mesh can mint a valid ticket using its own `NodeAddr`.

### DHCP coordination

Every router node runs dnsmasq and participates in DHCP lease assignment. CRDT
conflict-free merge prevents duplicate address assignment across concurrent allocations.
No single DHCP server — any router can hand out leases from its assigned subnet range.

**Centralization:** None for ongoing operation. New subnet ranges require coordination
(manual or future auto-assignment).

### DNS

DNS records are replicated via CRDT across all nodes. No single authoritative server.
Each node answers DNS queries for the mesh domain using its local CRDT replica.

**Centralization:** None structurally. Propagation lag means a freshly added record
may not be visible on all nodes immediately (eventual consistency).

### Routing

Subnet ownership is CRDT-synced (`/subnets/{cidr}` claim ledger). Each router redistributes its own subnet via Babel (`babeld`), and Babel computes loop-free cross-site routes over per-peer Iroh tunnels. A node going offline causes Babel to withdraw its routes within seconds; other subnets are unaffected. See `babel-routing.md`.

**Centralization:** None. Routes are additive CRDTs — removal requires tombstoning,
which is propagated on rejoin.

### Service discovery

Service registrations are CRDT-synced and tied to device leases. When a device's
lease expires, its service entries are cleaned up.

**Centralization:** None. Any node can answer service discovery queries.

### Gossip transport (iroh-gossip)

iroh-gossip is already fully P2P. Once peers have joined a topic, the gossip mesh is
self-healing — no bootstrap peer is needed for ongoing communication.

**Centralization:** Bootstrap only. The first join requires a known peer address (from
the ticket). After that, gossip propagates peer addresses transitively.

### Iroh relay infrastructure

Iroh uses n0's relay servers for NAT traversal fallback when direct connections fail.
This is an external dependency on n0's infrastructure (or a self-hosted relay).

**Centralization:** Real but bounded. Relay is used only for connection establishment,
not for data. If n0's relays are unreachable, direct connections still work where NAT
allows. Self-hosted relay is supported by Iroh.

---

## Failure Scenarios

| Scenario | Effect |
|----------|--------|
| Router goes offline | Other routers keep serving DHCP/DNS/routing. CRDT state is fully replicated — no data loss. |
| Network partition | Each partition operates independently with full CRDT state. On rejoin, CRDTs merge automatically. |
| Daemon crash | systemd restarts the daemon. Anti-entropy sync rebuilds any missed CRDT updates from peers on reconnect. |
| All routers restart simultaneously | CRDT state rebuilds from disk (if persisted) or anti-entropy from peers. New joins blocked until at least one router rejoins gossip. |
| Bootstrap peer unreachable | Multi-address ticket provides fallback peers. If all ticket peers are gone, out-of-band re-sharing needed. |
| n0 relay unreachable | Direct connections unaffected. NAT-traversal-dependent connections fall back to relay-less paths or fail. |

The critical remaining gap: **if all peers with valid tickets are offline simultaneously,
new nodes cannot join** until at least one existing peer comes back online with a
reachable address. This is inherent to any ticket-based bootstrap.

---

## What Pure P2P Requires

A fully decentralized mesh needs:

1. **Any peer can bootstrap new joiners.** Every participant should be able to produce
   a valid join ticket. This is already implemented.

2. **Multi-address tickets.** A ticket with N addresses succeeds if any 1 of N is
   reachable. Already implemented in ticket.rs.

3. **Distributed coordination state.** DHCP, DNS, routing, and service discovery all
   use CRDT replication — no central coordinator. Already implemented.

4. **Peer-to-peer discovery without a fixed bootstrap.** For truly infrastructure-free
   operation: DHT (iroh supports mainline DHT), mDNS for local networks, or PSK-derived
   topic IDs that any pre-authorized node can compute independently.

5. **Graceful departure.** When a peer leaves, its CRDT tombstones propagate so the
   rest of the mesh can clean up its leases and routes. Partially implemented via lease
   TTLs.

---

## Implementation Roadmap

### Phase 1: Multi-address tickets — DONE

ticket.rs now carries `Vec<NodeAddr>`. Any peer in the mesh can mint a ticket using
its own node address. Joiners try each address in order.

### Phase 2: Unified join flow (mesh.rs) — DONE

mesh.rs has `enter_room()` which handles both the "first node" case (no bootstrap peers)
and the "joining" case (bootstrap from ticket addrs). The host/join asymmetry at the
protocol level is gone.

### Phase 3: Mesh lib extraction — PLANNED

Extract a generic stream interface from room.rs so the mesh core (gossip, ticket, peer
management) can be used independently of the VPN coordination layer. This enables
embedding the mesh library in other Mjolnir components (e.g., guest agent peer
communication) without taking the full VPN stack.

### Phase 4: DHCP/DNS/routing coordination — PLANNED

Full CRDT-based network coordination as described in mesh-network-coordination.md and
mesh-network-crdt.md. Key work items:
- CRDT merge on gossip message receive
- Anti-entropy sync on peer reconnect
- Lease TTL expiry + tombstone propagation
- dnsmasq config generation from CRDT state

### Phase 5: Route persistence & offline resilience — FUTURE

Store CRDT state to disk so a restarting node can serve DHCP/DNS immediately without
waiting for anti-entropy. Store-and-forward for messages to temporarily offline nodes.
Explore DHT-based room discovery (iroh mainline DHT) to eliminate the ticket
requirement entirely for well-known mesh names.

---

## Summary

| Layer | Centralization | Status |
|-------|---------------|--------|
| Ticket bootstrap | Any peer can mint; multi-addr fallback | Done |
| DHCP coordination | CRDT, no single server | Planned |
| DNS | CRDT-replicated | Planned |
| Routing | Babel over per-peer Iroh tunnels; CRDT for subnet claims only | Planned |
| Service discovery | CRDT, tied to leases | Planned |
| Gossip | iroh-gossip, fully P2P | Done |
| NAT traversal | n0 relay (external dep) | Accepted / self-hostable |

The CRDT coordination work (Phases 4-5) is the remaining gap between the current
implementation and a fully resilient mesh. Phases 1-2 are complete; the gossip and
connection layers are already P2P.
