---
title: Rayfish Transport Hardening
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, rayfish, transport, overlay-filter]
sources: [public/rayfish/src/transport.rs]
state: [FUTURE]
confidence: high
---

# Rayfish Transport Hardening

Adopt rayfish's transport configuration patterns to harden Lightning Mesh's iroh endpoint setup.

## Concept

Rayfish has ~57K lines of transport hardening in `src/transport.rs` that mesh lacks. These are mostly drop-in patterns: overlay address filtering, versioned ALPN, warm hints, loss-tolerant QUIC, loop prevention, and graceful error classification.

## Patterns to Adopt

### 1. Overlay Address Filter

Rayfish drops VPN overlay IPs (`100.64.0.0/10`, `200::/7`) from iroh's direct-address candidates. This prevents routing loops when running alongside another VPN. Mesh needs the same protection.

**Rayfish implementation** (`transport.rs` line 455-483):

```rust
struct OverlayAddrFilter;

impl DirectAddrFilter for OverlayAddrFilter {
    fn keeps(&self, ip: IpAddr) -> bool {
        keeps_addr(ip, family_can_egress)
    }
}
```

Drops rayfish overlay IPs, foreign overlay IPs (Tailscale ULA `fd7a:115c:a1e0::/48`), and globally unroutable families. Keeps LAN paths even when global route is down.

**Mesh equivalent:** Replace in mesh's iroh endpoint builder. ~30 lines.

### 2. Versioned ALPN with Backwards Compatibility

Rayfish uses versioned mesh ALPN (`rayfish/mesh/7`) and accepts v5, v6 during rollouts. Dialing falls back to older versions automatically. `is_alpn_mismatch()` classifies dial errors to distinguish version mismatch from network failure.

**Rayfish implementation** (`transport.rs` line 111-114, 649-671):

```rust
pub const MESH_PROTOCOL_VERSION: u32 = 7;
pub const MESH_BACKWARDS_COMPAT: &[u32] = &[6, 5];

pub fn is_alpn_mismatch(err: &str) -> bool {
    let e = err.to_lowercase();
    e.contains("known protocol") || e.contains("application protocol")
}
```

**Mesh equivalent:** Mesh's CRDT sync uses iroh-gossip without version negotiation. Adding versioned ALPN helps mesh nodes gracefully handle version incompatibilities.

### 3. Loss-Tolerant QUIC Configuration

Rayfish tunes QUIC for datagram-heavy forwarding: disables send fairness (no competing streams), disables GSO on Linux (stall recovery issue), uses small datagram send buffer (16KB, discards old on overflow), applies `LossTolerantConfig` congestion controller.

**Rayfish implementation** (`transport.rs` line 555-577):

```rust
pub(crate) fn quic_transport_config(engine: QuicEngine) -> QuicTransportConfig {
    QuicTransportConfig::builder()
        .congestion_controller_factory(Arc::new(LossTolerantConfig::default()))
        .send_fairness(false)
        .enable_segmentation_offload(MESH_QUIC_SEGMENTATION_OFFLOAD)
        .datagram_send_buffer_size(datagram_send_buffer_size(engine))
        .build()
}
```

**Mesh equivalent:** Mesh's TUN traffic has similar datagram-heavy characteristics. Same tuning applies.

### 4. Control Plane DNS Isolation

Rayfish never asks its own Magic DNS for control plane lookups. It builds an explicit DNS resolver for iroh's infrastructure names (relay, pkarr), filters out overlay addresses, caps at 4 nameservers, keeps both IP families, and falls back to public DNS (Cloudflare 1.1.1.1, Quad9 9.9.9.9, Google 8.8.8.8, 8.8.4.4) when the host has no resolver.

**Rayfish implementation** (`transport.rs` line 159-217):

```rust
pub(crate) const PUBLIC_FALLBACK_DNS: [IpAddr; 4] = [
    IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
    IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)),
    IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
    IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888)),
];
```

**Mesh equivalent:** Mesh's iroh endpoint uses default system DNS. If a mesh node's resolv.conf points at its own DNS, a restart waits on the data plane it is trying to bring up.

### 5. Loop Prevention for Exit Nodes

Rayfish's `LoopPrevention` is a `SocketConfigurator` that marks iroh's own sockets to prevent routing loops when the exit node takes the default route through the TUN.

**Mesh equivalent:** Mesh's overlay TUN (`mjolnir0`) could benefit from the same pattern if mesh adds exit node support.

## Benefits

- Prevents routing loops when mesh runs alongside other VPNs
- Graceful version negotiation between mesh nodes
- Better QUIC performance for TUN datagram traffic
- Control plane survives host DNS failure
- Prevents iroh traffic from looping through the TUN

## Risks

- Transport patterns are iroh-specific (both stacks use iroh, so low risk)
- Some patterns (loss-tolerant config, GSO settings) are Linux-specific

## Effort

Low. Most patterns are 20-100 lines each. Overlay address filter alone is ~30 lines. Total: ~200-300 lines of high-value code.
