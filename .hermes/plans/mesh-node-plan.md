# Plan: Stand up a Lightning Mesh node

## Goal
Write a standalone iroh mesh node that connects to the existing mjolnir-mesh private network and exchanges gossip with other peers.

## Scope
- A minimal Rust crate that uses iroh to join the mesh
- Connects to peers from the fleet roster
- Exchanges CRDT gossip messages (subnet claims, leases, services)
- Can run on macOS for testing (no TUN required)

## Phase 0: Source Review (DONE)
- Read mjolnir-meshd.rs (11.5k lines) — understood iroh endpoint setup, gossip sync, TUN encap
- Read crdt/sync.rs — understood GossipTransport trait, GossipSync dispatcher, postcard encoding
- Read crdt/gossip.rs — understood GossipMessage enum variants (15 types)
- Read capability_beacon.rs — understood signed discovery locators, handshake protocol
- Read bootstrap.rs — understood rank_bootstrap_candidates (4-tier peer ranking)
- Read roster.rs — understood PeerRoster file format (64-hex node ids)
- Read transport.rs — understood HouseDialGate, OuterGuard, supervised_mesh lifecycle
- Read Cargo.toml — understood daemon feature flag, iroh dependency versions

## Phase 1: Create the node crate
### File: crates/mjolnir-node/Cargo.toml
- New crate: `mjolnir-node`
- Dependencies: iroh (same version as mjolnir-mesh), clap, serde, tokio, postcard, bytes
- Binary: `mjolnir-node`

### File: crates/mjolnir-node/src/main.rs
- Parses CLI args: `--roster <file>`, `--id <name>`, `--lan`
- Loads or creates Ed25519 secret key
- Builds iroh Endpoint (N0 preset or Minimal for LAN)
- Joins the mesh gossip topic
- Runs the gossip receive loop
- Prints received gossip messages to stdout

### File: crates/mjolnir-node/src/roster.rs
- Reuse PeerRoster from mjolnir-mesh (or vendored copy)
- Parse roster file: one 64-hex node id per line

### File: crates/mjolnir-node/src/topic.rs
- mesh_topic_id() — same constant hash as mjolnir-mesh
- TopicId derivation for gossip joins

## Phase 2: Wire up gossip
### File: crates/mjolnir-node/src/gossip.rs
- GossipTransport impl over iroh-gossip
- GossipSync dispatch loop
- Postcard encode/decode of GossipMessage variants
- Handler trait for received messages

### File: crates/mjolnir-node/src/endpoint.rs
- build_endpoint() — mirrors mjolnir-meshd's endpoint setup
- LAN mode: Minimal preset, no relay, mDNS lookup
- Internet mode: N0 preset, relay, pkarr/DNS
- Secret key persistence

## Phase 3: Fleet integration
### File: crates/mjolnir-node/src/fleet.rs
- Load fleet-nodes.conf from deploy/openwrt/
- Parse node IDs and group membership
- Filter active nodes for bootstrap

## Phase 4: Testing
- cargo test --workspace
- Test with 2 nodes on same LAN
- Verify gossip convergence

## Trade-offs
- Minimal vs full meshd: this node does NOT run babeld, TUN, or DNS — it's a gossip-only observer
- Uses same iroh versions as mjolnir-mesh for wire compat
- Postcard encoding must match exactly — reuse mjolnir-mesh's GossipMessage types

## Timeline
- Phase 1: 1 task
- Phase 2: 2 tasks
- Phase 3: 1 task
- Phase 4: 1 task
