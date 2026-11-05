---
title: Rayfish Overview
created: 2026-11-05
updated: 2026-11-05
type: concept
tags: [interop, rayfish, vpn]
sources: [public/rayfish/src/]
state: [CURRENT]
confidence: high
---

# Rayfish Overview

Rayfish is a P2P mesh VPN built on iroh. Peers are addressed by cryptographic identity, not IP. Dual-stack addressing: stable IPv4 in `100.64.0.0/10` (FNV-1a of identity), stable IPv6 in `200::/7` (blake3, never rotates). **Overlay is IPv6-only.** IPv4 only for exit node egress.

## Key Facts

- **Version:** 0.5.15 (experimental, pre-1.0)
- **License:** MPL-2.0
- **Language:** Rust 2024, MSRV 1.91
- **Scale:** 206 Rust files, ~132K lines, 4 workspace members
- **Transport:** One iroh endpoint + one TUN, shared across all networks
- **Protocol version:** 7 (supports v5, v6 backwards)

## Architecture

```
App -> TUN (100.64.x.x / 200::x) -> rayfish -> iroh QUIC datagrams -> peer
```

One QUIC connection per peer device. Network selected in-band via `ControlFrame.net` and 2-byte network-handle tags.

## Core Services

- **NetworkRegistry** -- networks map, membership, coordinator, admission
- **ConnectionManager** -- per-peer QUIC connection + frame demux
- **DnsService** -- `.ray` Magic DNS responder (45K lines)
- **FileService** -- peer-to-peer file transfer (`FILES_ALPN`)
- **ConnectService** -- direct links, multi-device identity (`CONNECT_ALPN`)
- **Firewall** -- stateful conntrack, per-peer rules, secure-by-default

## Design Invariants

- Reachability = shared network. Peers exchange packets iff they share at least one network.
- Room id is a discovery key, never a credential.
- Signed `GroupBlob` is source of truth. One pkarr record per network.
- Coordinator is network-key holder. Any holder can admit, suggest firewall, kick, republish.
- Firewall is secure-by-default: deny inbound TCP/UDP, allow inbound ICMP, allow outbound.
- Data plane vs control plane: `up`/`down` toggles only data plane. `start`/`stop` toggles the whole process.

## CLI

`ray up | down | start | stop | restart | create | join | leave | nuke | ephemeral | invite | firewall | exit-node | connect | ping | status | send | files | config | hostname | admin | pair | unpair | rootless | update | ssh`

## Privilege Model

Tailscale operator model. Always-root daemon does privileged work (TUN, DNS, routes). Clients are unprivileged, talk via Unix-socket IPC. Authority is `SO_PEERCRED` UID check.

## Key Files

- `src/transport.rs` -- iroh endpoint setup, ALPN versioning, overlay address filtering, QUIC config, control plane DNS isolation (57K lines)
- `src/forward.rs` -- TUN packet forwarding, per-peer routing, fragmentation, FqCodel (75K lines)
- `src/firewall.rs` -- stateful firewall with conntrack, per-peer rules, rule origins (56K lines)
- `src/identity.rs` -- stable Ed25519 SecretKey persistence, device certs (160 lines)
- `src/dns/mod.rs` -- Magic DNS with `.ray` domain, DoT forwarding, overlay IP filtering (39K lines)
- `src/invite.rs` -- single-use invite codes, InviteStore, coordinator admission (35K lines)
- `src/membership.rs` -- Member roster, derive_ipv6(), ExitFamilies, Peer resolution (1,928 lines)
- `src/peers.rs` -- PeerTable, DeviceUserMap, reachability tracking (84K lines)
