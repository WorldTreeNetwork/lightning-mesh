---
title: Rayfish Firewall on Mesh Overlay
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, rayfish, firewall, security]
sources: [public/rayfish/src/firewall.rs]
state: [FUTURE]
confidence: medium
---

# Rayfish Firewall on Mesh Overlay

Add a stateful firewall to Lightning Mesh's overlay TUN, using rayfish's firewall model.

## Concept

Lightning Mesh has zero firewall on the overlay TUN. Any node can reach any other without filtering. Rayfish has a mature stateful firewall with conntrack, per-peer rules, network-scoped rules, and secure-by-default posture.

## Rayfish Firewall Architecture

Rayfish's firewall (`src/firewall.rs`, ~1,200 lines) inspects IP packets carried inside the VPN. It does NOT filter the control plane (QUIC streams for Welcome, MemberSync, MeshHello). The firewall only sees TUN-bound packets.

**Default posture:**

| Traffic | Default |
|---|---|
| Inbound ICMP (v4+v6) | allow (so ping works) |
| Inbound TCP | deny (no listening port exposed) |
| Inbound UDP | deny |
| Outbound (any proto) | allow |
| Return traffic | allow (conntrack) |

**Key types:**

```rust
pub struct FirewallRule {
    pub direction: Direction,     // In or Out
    pub action: Action,           // Allow or Deny
    pub protocol: Protocol,       // Tcp, Udp, Icmp, Any
    pub port: Option<PortRange>,  // None = all ports
    pub peer: PeerFilter,         // Any, Identity(id), Except{...}
    pub network: Option<String>,  // Scope to a network
    pub origin: RuleOrigin,       // Local or Network(net)
}
```

**Connection tracking:**

- TCP: 300s idle timeout, 30s closing window
- UDP: 30s timeout
- ICMP: echo request starts a flow, echo reply matches via icmp_id
- RST removes tracking immediately
- Flow eviction runs every 60s

**Rule origins:**

- `RuleOrigin::Local` -- hand-added by the node, never touched by network reconvergence
- `RuleOrigin::Network(net)` -- from coordinator suggestions, replaced atomically on each blob update
- `RuleOrigin::Ssh` -- auto-seeded SSH passthrough, removed on `ssh off`

**REJECT mode:** opt-in. Sends synthetic TCP RST or ICMP unreachable instead of silent drop.

## Mesh Integration Options

### Option A: Feature-Gated Module

Add a `firewall` module to `mjolnir-mesh` with the same architecture:

```rust
pub struct SharedFirewall {
    inner: Arc<ArcSwap<FirewallConfig>>,  // lock-free reads
    conntrack: Arc<FastDashMap<Flow, FlowState>>,  // stateful tracking
    edits: Arc<Mutex<()>>,  // serialized writes
}
```

Plugs into the overlay TUN write path in `forward.rs` (or mesh equivalent).

**Pros:**
- Minimal code change to mesh
- Same API surface as rayfish (familiar to operators)
- Can be enabled/disabled per-node

**Cons:**
- 1,200+ lines to port/adapt
- Firewall rules need a storage mechanism (file, CRDT, both?)

### Option B: CRDT-Synced Firewall

Firewall rules are CRDT entries (like ServiceBook). All nodes converge on the same ruleset via gossip.

**Pros:**
- Consistent policy across the mesh
- No per-node configuration drift
- Works with mesh's existing sync model

**Cons:**
- More complex CRDT design (need conflict resolution for conflicting rules)
- Rule propagation latency (gossip tick, not instant)
- Risk of malicious rule injection via gossip

### Option C: Hybrid -- Local + Network Rules

Keep rayfish's `RuleOrigin` model: local rules are per-node, network rules are CRDT-synced.

**Pros:**
- Best of both worlds
- Operators can override global policy locally
- Matches rayfish's proven model

**Cons:**
- Most complex to implement

## Configuration Storage

Rayfish uses `firewall.toml` on disk. Mesh could use:

- File-based (same as rayfish): simple, per-node
- CRDT-based: synced across mesh, consistent
- Both: file for local overrides, CRDT for network defaults

## Benefits

- Secure-by-default overlay: new mesh nodes cannot accidentally expose services
- Per-peer port filtering: allow SSH from trusted nodes, deny from others
- Network-scoped rules: different policies for different mesh networks
- Stateful conntrack: return traffic auto-allowed
- REJECT mode: faster failure detection for denied connections

## Risks

- 1,200+ lines of new code
- Firewall misconfiguration could isolate nodes
- CRDT-synced rules introduce attack surface (gossip injection)
- Performance impact on TUN forwarding path (needs to be hot-path friendly)

## Effort

Medium-High. Core firewall logic is ~800 lines (after removing rayfish-specific types). Conntrack is ~200 lines. Integration with mesh's TUN path is ~100 lines.
