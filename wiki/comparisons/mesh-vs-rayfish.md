---
title: Lightning Mesh vs Rayfish
created: 2026-11-05
updated: 2026-11-05
type: comparison
tags: [interop, rayfish, mesh-core, vpn]
sources: [crates/mjolnir-mesh/, public/rayfish/src/]
state: [CURRENT]
confidence: high
---

# Lightning Mesh vs Rayfish

| Dimension | Lightning Mesh | Rayfish |
|---|---|---|
| **Language** | Rust | Rust 2024 |
| **Runtime** | Static binary (OpenWrt) | Static binary (Linux/macOS/Android/iOS) |
| **License** | AGPL-3.0 (dual: commercial) | MPL-2.0 |
| **Mission** | Mesh OS (CRDT state, Babel routing, hello.mesh) | P2P VPN (TUN + Magic DNS + firewall) |
| **Transport** | Iroh (QUIC + NAT traversal) | Iroh (QUIC + NAT traversal) |
| **State Model** | CRDT (FWW merge) | Signed GroupBlob (pkarr) |
| **Addressing** | blake3 IDs, /24 claims from 10.42.0.0/16 | Deterministic: IPv4 100.64.0.0/10, IPv6 200::/7 |
| **Crypto** | Curve25519, blake3, postcard | Curve25519, blake3, FNV-1a, msgpack |
| **Routing** | Babeld (external process) | Per-peer routing via QUIC datagrams |
| **Backhaul** | 802.11s WiFi mesh | iroh QUIC (NAT traversal, relays) |
| **Overlay** | TUN device (mjolnir0) | TUN device (100.64.x.x / 200::x) |
| **Service Discovery** | .mesh DNS + gossip CRDT | .ray Magic DNS (45K lines) |
| **Identity** | Iroh EndpointId (Curve25519) | Iroh EndpointId (Curve25519) + device certs |
| **Firewall** | None | Stateful conntrack, per-peer rules |
| **Protocol Version** | Not versioned | Versioned ALPN (v7, supports v5, v6) |
| **Min Bandwidth** | ~1 Mbps (WiFi backhaul) | Depends on QUIC (varies) |
| **Min MTU** | ~1500 bytes | ~1280 bytes (QUIC datagram) |
| **Target Hardware** | OpenWrt routers ($30-80) | Linux, macOS, Android, iOS |
| **Field Status** | 5-router fleet, July 2026 | Experimental, pre-1.0 |
| **Scale** | ~55K LOC, 7 crates | ~132K LOC, 206 files, 4 crates |
| **Multi-device** | No (single owner) | Yes (user signs device key) |
| **Exit Nodes** | Partial (via firewall/nftables) | Full (client + gateway mode) |
| **Magic DNS** | .mesh (simple) | .ray (comprehensive, DoT) |
| **Firewall Rules** | None | Per-peer, per-network, local override |
| **Invite System** | Owner-claim (device proof) | Signed invite + GroupBlob + nullifiers |

## Key Similarities

- Both use iroh (QUIC + NAT traversal) as transport
- Both use Curve25519 for identity
- Both use blake3 for address derivation
- Both use a TUN device for overlay networking
- Both run as static Rust binaries
- Both support multi-network (mesh guilds / rayfish networks)

## Key Differences

### Mesh Strengths

- **Field-validated** on OpenWrt routers (5-router fleet)
- **CRDT state model** -- conflict-free replication, no central authority
- **Rich service layer** -- hello.mesh directory, .mesh DNS, mini-apps, ACME HTTPS
- **Operational tooling** -- mjolnir-apply for staged updates, Lightning Admin desktop
- **Babel routing** -- proven IP routing over mesh backhaul
- **Lower code volume** -- 55K LOC vs 132K LOC

### Rayfish Strengths

- **Mature firewall** -- stateful conntrack, per-peer rules, secure-by-default
- **Deterministic addressing** -- IPs derived from identity, never change
- **Multi-device model** -- user signs device key, network-scoped admission
- **Magic DNS** -- comprehensive .ray resolver with DoT forwarding
- **Exit nodes** -- full client + gateway mode with loop prevention
- **Versioned protocol** -- ALPN versioning with backwards compatibility
- **Larger codebase** -- more battle-tested patterns (132K LOC)

## Complementarity

The stacks are highly complementary:

1. **Mesh brings CRDT state and operational maturity** to the mesh OS domain
2. **Rayfish brings firewall, deterministic addressing, and device hierarchies** that mesh lacks
3. **Shared transport** (iroh) and **shared crypto** (Curve25519, blake3) enable deep integration
4. **Transport-agnostic gossip seam** in mesh enables swapping in rayfish components

## Integration Priority

See the interop pages for detailed integration strategies:

- [[interop-rayfish-transport]] -- Transport hardening patterns (Phase 1, HIGH VALUE)
- [[interop-rayfish-addressing]] -- Deterministic addressing from identity (Phase 1, HIGH VALUE)
- [[interop-rayfish-firewall]] -- Stateful firewall on overlay TUN (Phase 2, HIGH VALUE)
- [[interop-rayfish-dns]] -- Magic DNS patterns (Phase 2, MEDIUM VALUE)
- [[interop-rayfish-invite]] -- Invite and admission model (Phase 3, LOW-MEDIUM VALUE)
