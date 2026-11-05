---
title: Rayfish Codebase Grounding
created: 2026-11-05
updated: 2026-11-05
type: research
tags: [rayfish, codebase, grounding, interop]
sources: [public/rayfish/src/identity.rs, public/rayfish/src/membership.rs, public/rayfish/src/transport.rs, public/rayfish/src/firewall.rs, public/rayfish/src/forward.rs, public/rayfish/src/dns/mod.rs, public/rayfish/src/invite.rs, public/rayfish/src/peers.rs, public/rayfish/src/lib.rs]
state: [CURRENT]
confidence: high
---

# Rayfish Codebase Grounding

This page documents the actual Rayfish source code to ground all wiki interop planning in verified implementation details.

## Repository Overview

| Property | Value |
|---|---|
| Language | Rust 2024, MSRV 1.91 |
| LOC | ~132K lines (Rust) |
| Files | 206 `.rs` files |
| License | MPL-2.0 |
| Version | 0.5.15 (experimental, pre-1.0) |
| Workspace | `rayfish`, `ray-proto`, `ray-mobile`, `ray-apple` |

## Module Structure (src/lib.rs)

```
rayfish/src/
  lib.rs                  -- Module declarations, AsyncMutex alias, APP_NAME="ray"
  identity.rs             -- Ed25519 SecretKey persistence, device certs (160 lines)
  membership.rs           -- Member roster, IP derivation, exit families (1,928 lines)
  transport.rs            -- iroh endpoint, ALPN, QUIC config, DNS isolation (57K lines)
  forward.rs              -- TUN packet forwarding, per-peer routing, fragments (75K lines)
  firewall.rs             -- Stateful conntrack, per-peer rules (56K lines)
  dns/mod.rs              -- Magic DNS .ray resolver, DoT forwarding (39K lines)
  invite.rs               -- Single-use invite codes, coordinator admission (35K lines)
  peers.rs                -- PeerTable, DeviceUserMap, reachability (84K lines)
  daemon/                 -- NetworkRegistry, mesh daemon, IPC
  control/                -- Control messages, IPC protocol
  config/                 -- Configuration, quic engine selection
  tun/                    -- TUN device management
  exit_node/              -- Exit node client + gateway
  ssh/                    -- Embedded SSH server
  dht/                    -- iroh DHT interaction
  keybackup/              -- Encrypted key backup
```

## Identity Model (identity.rs)

### Key Storage

```rust
// File: ~/.config/rayfish/secret_key
// Format: raw 32-byte Ed25519 private key
// Type: iroh::SecretKey (which is Ed25519)

pub fn load_existing() -> Result<Option<SecretKey>> {
    let bytes: [u8; 32] = std::fs::read(&path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("corrupt secret key file"))?;
    Ok(Some(SecretKey::from_bytes(&bytes)))
}
```

### Key Points

- **Single key**: Rayfish uses ONE `iroh::SecretKey` (Ed25519, 32 bytes) for all identity
- **iroh::EndpointId** = `public key` derived from `SecretKey`
- **No X25519 separate key**: Unlike Reticulum's 64-byte key, Rayfish uses iroh's single key which serves both authentication and key agreement
- **`load_existing()` vs `load_or_create()`**: Critical distinction — `load_existing()` returns `None` without minting a key (for restore scenarios), `load_or_create()` auto-generates if absent

### Device Certificates

```rust
pub struct DeviceCert {
    pub user_identity: SecretKey,  // User's iroh::SecretKey
    pub device_key: PublicKey,      // Device's iroh::SecretKey.public()
    pub signature: Signature,       // User signed device_key
}
```

- Stored at `~/.config/rayfish/device_cert`
- Serialized as msgpack (`rmp_serde`)
- Verified on reload: `cert.verify()` checks the signature
- Deleted on unpair: `delete_device_cert()` is idempotent

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Key type | Ed25519 (iroh::SecretKey) | Curve25519 (iroh::SecretKey) |
| Key size | 32 bytes | 32 bytes |
| Storage | `~/.config/rayfish/secret_key` | `~/.config/mjolnir/identikey.json` |
| Device certs | Yes (user signs device) | No |
| Key restore | `store_secret_key()` replaces | `keyspace.change` |

## Address Derivation (membership.rs)

### IPv6 Derivation — `derive_ipv6()`

```rust
pub fn derive_ipv6(identity: &EndpointId) -> Ipv6Addr {
    let mut hex = [0u8; 64];
    hex::encode_to_slice(identity.as_bytes(), &mut hex)
        .expect("a 32-byte key always encodes to exactly 64 hex digits");
    let hash = blake3::hash(&hex);
    let bytes = hash.as_bytes();
    let octets: [u8; 16] = [
        0x02, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
        bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
    ];
    Ipv6Addr::from(octets)
}
```

**Algorithm:**
1. Encode `EndpointId` as lowercase hex (64 chars)
2. Hash with `blake3` (32 bytes)
3. Take first 15 bytes
4. Prepend `0x02` → `200::/7` range

**Properties:**
- **120-bit address space** (2^120) — collisions "practically impossible"
- **Zero allocation** — hex encoding done on stack (`[u8; 64]`)
- **Never carried on the wire** — every node computes every peer's address
- **Stable across reboots** — same identity always maps to same IPv6

### IPv4 Derivation

- Range: `100.64.0.0/10` (CGNAT range)
- Algorithm: FNV-1a hash of identity → 32-bit IPv4
- Note: Rayfish is IPv6-only on the overlay. IPv4 is only for exit node egress.

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| IPv6 | `200::/7` via blake3(hex(identity)) | Not used (IPv4 only) |
| IPv4 | `100.64.0.0/10` via FNV-1a | `10.42.0.0/16` via HLC claims |
| Derivation | Deterministic from identity | First-writer-wins /24 claims |
| Wire transport | Addresses never on wire | SubnetClaim CRDT carries claims |
| Conflicts | None (hash-based) | HLC timestamp resolution |

## Member Roster (membership.rs)

### Member Struct

```rust
pub struct Member {
    pub identity: EndpointId,        // Device's iroh::EndpointId
    pub is_coordinator: bool,
    pub hostname: Option<String>,
    pub user_identity: Option<EndpointId>,  // Paired user key
    pub device_cert: Option<DeviceCert>,
    pub last_seen: Option<u64>,      // Unix seconds (disconnect timestamp)
    pub exit_node: bool,
    pub exit_families: ExitFamilies, // ?, 4, 6, d, n
}
```

### ExitFamilies Enum (wire-optimized)

```rust
pub enum ExitFamilies {
    Unknown,  // serialized as "?" (1 byte)
    V4,       // serialized as "4"
    V6,       // serialized as "6"
    Dual,     // serialized as "d"
    Neither,  // serialized as "n"
}
```

- **Three-valued**: Unknown is distinct from "no" for backward compatibility
- **One-char serialization**: msgpack writes unit variants as their name; short names save 8 bytes per member on the wire
- **`tunnelled()`**: Unknown → V6 (older gateways keep IPv6 behavior by default)

### MemberList

- `BTreeMap<EndpointId, Member>` — ordered by identity
- Serialized as plain array (positional msgpack — field order IS the wire format)
- New fields appended at end for forward compatibility
- Wire is array-encoded: a skipped field is a missing slot, not an absent key

### Peer Resolution

```rust
pub fn resolve_peer_literal(&self, name: &str) -> Option<EndpointId> {
    // 1. Try as IPv6 address → derive from each member's identity
    // 2. Try as EndpointId → match against identity or user_identity
}
```

## Firewall (firewall.rs)

### Default Posture

| Traffic | Default |
|---|---|
| Inbound ICMP (v4+v6) | allow |
| Inbound TCP | deny |
| Inbound UDP | deny |
| Outbound (any proto) | allow |
| Return traffic | allow (conntrack) |

### FirewallRule Struct

```rust
pub struct FirewallRule {
    pub direction: Direction,     // In or Out
    pub action: Action,           // Allow or Deny
    pub protocol: Protocol,       // Tcp, Udp, Icmp, Any
    pub port: Option<PortRange>,  // None = all ports
    pub peer: PeerFilter,         // Any, Identity(id), Except{...}
    pub network: Option<String>,  // Scope to a network
    pub origin: RuleOrigin,       // Local, Network(net), or Ssh
}
```

### Connection Tracking

- **TCP**: 300s idle timeout, 30s closing window, RST removes immediately
- **UDP**: 30s timeout
- **ICMP**: echo request starts flow, echo reply matches via icmp_id
- **Eviction**: runs every 60s
- **REJECT mode**: opt-in, sends synthetic TCP RST or ICMP unreachable

### Rule Origins

- `RuleOrigin::Local` — hand-added, never touched by network reconvergence
- `RuleOrigin::Network(net)` — from coordinator suggestions, replaced atomically
- `RuleOrigin::Ssh` — auto-seeded SSH passthrough, removed on `ssh off`

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Firewall | Stateful, per-peer, per-network | None |
| Conntrack | TCP/UDP/ICMP with timeouts | N/A |
| Rule storage | `firewall.toml` + coordinator | N/A |
| REJECT mode | Synthetic RST/ICMP unreachable | N/A |

## Transport (transport.rs)

### iroh Endpoint Setup

- **One iroh endpoint** shared across ALL networks
- **ALPN versioning**: `rayfish/mesh/7`, accepts v5, v6 during rollouts
- **Protocol version**: 7 (supports v5, v6 backwards)
- **`is_alpn_mismatch()`**: classifies dial errors

### Overlay Address Filter

```rust
struct OverlayAddrFilter;
impl DirectAddrFilter for OverlayAddrFilter {
    fn keeps(&self, ip: IpAddr) -> bool {
        // Drops: 100.64.0.0/10, 200::/7, Tailscale ULA fd7a:115c:a1e0::/48
        // Keeps: LAN paths even when global route is down
    }
}
```

### QUIC Configuration

```rust
pub fn quic_transport_config(engine: QuicEngine) -> QuicTransportConfig {
    QuicTransportConfig::builder()
        .congestion_controller_factory(Arc::new(LossTolerantConfig::default()))
        .send_fairness(false)          // No competing streams
        .enable_segmentation_offload(MESH_QUIC_SEGMENTATION_OFFLOAD)
        .datagram_send_buffer_size(datagram_send_buffer_size(engine)) // 16KB
        .build()
}
```

### Control Plane DNS Isolation

```rust
pub const PUBLIC_FALLBACK_DNS: [IpAddr; 4] = [
    1.1.1.1, [2606:4700:4700::1111],  // Cloudflare
    8.8.8.8, [2001:4860:4860::8888],  // Google
];
```

- Never asks own Magic DNS for control plane lookups
- Filters overlay addresses from host resolvers
- Falls back to public DNS when host has none
- Capped at 4 nameservers, keeps both IP families

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Endpoint | One shared across all networks | One per network? |
| ALPN | Versioned `rayfish/mesh/7` | Not versioned |
| QUIC | Loss-tolerant, 16KB datagram buf | Default iroh config |
| DNS | Explicit control plane resolver | System default |
| Overlay filter | Drops CGNAT/ULA from direct addrs | None |

## Forwarding (forward.rs)

### Data Plane Architecture

Three concurrent tasks:
1. **`run_mesh`**: reads outgoing packets from TUN, routes to correct peer via `PeerTable`
2. **`spawn_peer_reader`**: one per peer, reads incoming datagrams, forwards to TUN writer
3. **`spawn_tun_writer`**: single task, writes incoming packets to the TUN device

### Key Components

- **`PeerTable`**: maps IPv6 addresses to peer connections
- **`FragmentVerdicts`**: reassembles fragmented IPv6 packets
- **`FqCodel`**: active queue management per peer (fair queuing + CoDel)
- **`ForwardMetrics`**: drop reasons, byte counts, packet counts

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Tasks | 3 concurrent (TUN reader, peer readers, TUN writer) | Single loop |
| Fragmentation | IPv6 fragment reassembly | Not present |
| QoS | FqCodel per peer | None |
| Metrics | ForwardMetrics (drops, bytes, packets) | None |

## DNS (dns/mod.rs)

### Magic DNS Architecture

- **Magic IP**: `100.100.100.53` (v4), `200::53` (v6)
- **Domain**: `.ray` (DNS_DOMAIN constant)
- **No host port 53 bind**: DNS packets routed to TUN, intercepted by `forward::run_mesh`
- **`simple_dns` crate**: parses/builds DNS packets

### Resolver Flow

1. Client sends UDP DNS to `MAGIC_DNS_V6`
2. `forward::run_mesh` intercepts, hands to `resolver::Resolver`
3. Resolver checks `HostnameTable` for `.ray` names
4. Falls back to upstream DoT for non-`.ray` names
5. `HostnameTable`: `HashMap<network, HashMap<hostname, Ipv6Addr>>`
6. `ReverseLookupTable`: `DashMap<Ipv6Addr, (hostname, network)>`

### Key Behaviors

- **Inside `.ray`**: zone is authoritative — NXDOMAIN for unknown names (with 60s SOA TTL)
- **Outside `.ray`**: only answers when roster holds the name (prevents swallowing public names)
- **Short names**: `hostname.network.ray` and `hostname.network` both resolve
- **A queries**: always NODATA (overlay is IPv6-only)
- **PTR queries**: reverse lookup for `ip6.arpa` in `200::/7` range

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Domain | `.ray` | `.mesh` |
| Resolver IP | `200::53` (IPv6 only) | Not specified |
| Upstream | DoT forwarding | Not present |
| Short names | `hostname.network` resolves | Not present |
| NODATA for A | Yes (IPv6-only overlay) | N/A |
| Negative TTL | 60s SOA (our SOA, not root's) | N/A |

## Invite System (invite.rs)

### Invite Code Format

```
bs58(network_pubkey || coordinator || secret || checksum)
```

- **Network pubkey**: the network's iroh::EndpointId
- **Coordinator**: the issuing coordinator's iroh::EndpointId
- **Secret**: 32 random bytes
- **Checksum**: 4 bytes for error detection

### Invite Lifecycle

```
Pending → Redeemed (burned) | Revoked | Expired
```

- **Single-use**: `redeem()` burns the invite immediately
- **Expiry**: TTL-based (default configurable)
- **Hostname binding**: coordinator can assign hostname on redemption
- **Private delivery**: `delivery_for()` creates invite for specific recipient via ALPN
- **Multi-coordinator**: `record_shared()` lets co-coordinators share invite hashes

### InviteStore

- **Location**: `~/.config/rayfish/invites/<network>.toml`
- **Permissions**: 0600 (owner-only)
- **Operations**: mint, redeem, revoke, restore, burn_by_hash, list

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Admission | Signed invite + GroupBlob | Owner-claim (device proof) |
| Multi-coordinator | Yes (shared invite hashes) | Single owner |
| Revocation | Nullifier in GroupBlob | Full ownership transfer |
| Invite format | bs58 encoded | N/A |
| Private delivery | ALPN offer to specific peer | N/A |

## Wire Format

### GroupBlob (Roster)

- **Serialization**: msgpack, array-encoded (positional)
- **Content**: `Vec<Member>` — ordered by identity
- **Signed**: pkarr record (DNS TXT)
- **Forward compatibility**: new fields appended at end

### Control Messages

- **Serialization**: rmp_serde (msgpack + serde)
- **ALPN frames**: `ControlFrame.net` (2-byte network handle) + message type
- **Message types**: Welcome, MemberSync, SignedRecord, InviteOffer, etc.

### Mesh Comparison

| Aspect | Rayfish | Lightning Mesh |
|---|---|---|
| Roster format | msgpack array of Member | postcard CRDT entries |
| Signing | pkarr DNS TXT record | CRDT signatures |
| Control framing | 2-byte network handle + ALPN | iroh gossip channels |
| Wire protocol | rmp_serde | postcard |

## Key Files (Verified from Source)

- `src/identity.rs` (160 lines) — Ed25519 SecretKey, device certs, persistence
- `src/membership.rs` (1,928 lines) — Member roster, derive_ipv6(), ExitFamilies, MemberList
- `src/transport.rs` (57,258 lines) — iroh endpoint, ALPN, QUIC config, DNS isolation, overlay filter
- `src/forward.rs` (75,429 lines) — TUN forwarding, PeerTable, fragmentation, FqCodel
- `src/firewall.rs` (56,537 lines) — Stateful firewall, conntrack, per-peer rules
- `src/dns/mod.rs` (39,429 lines) — Magic DNS .ray resolver, DoT forwarding
- `src/invite.rs` (35,568 lines) — Invite codes, InviteStore, coordinator admission
- `src/peers.rs` (84,627 lines) — PeerTable, DeviceUserMap, reachability tracking
- `src/lib.rs` (138 lines) — Module declarations, APP_NAME, AsyncMutex

## Risks and Constraints (Verified from Code)

1. **iroh::SecretKey = Ed25519**: Rayfish uses ONE Ed25519 key for all identity. Unlike Reticulum's dual X25519+Ed25519 (64 bytes), there is no separate key agreement key.
2. **Posititional msgpack wire**: Field order in Member IS the wire format. Adding fields mid-struct shifts all subsequent fields. New fields must be appended.
3. **One iroh endpoint per process**: All networks share a single iroh endpoint. Network separation is in-band via ALPN + 2-byte handles.
4. **IPv6-only overlay**: The overlay is IPv6-only. IPv4 exists only for exit node egress and is NOT routed on any node.
5. **Addresses never on wire**: Every node derives every peer's IPv6 from their EndpointId. No address discovery protocol needed.
6. **Coordinator is mandatory**: Rayfish requires a coordinator (network key holder) for admission. Mesh's CRDT model is decentralized.
7. **40K-line invite module**: The invite system is tightly coupled to rayfish's daemon architecture. Pattern borrowing is the only viable integration path.
8. **MPL-2.0 license**: Rayfish is MPL-2.0. Mesh is AGPL-3.0. The MPL-2.0 parts can be incorporated into AGPL projects.
