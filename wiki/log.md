---
title: Wiki Log — Lightning Mesh
created: 2026-11-05
updated: 2026-11-05
type: summary
tags: [wiki-admin]
sources: [wiki/log.md]
state: [CURRENT]
---

# Wiki Log — Lightning Mesh

## 2025-11-05

- Created wiki directory structure: concepts/, comparisons/, interop/, appendix/, raw/, queries/
- Created SCHEMA.md with conventions and tag taxonomy
- Created index.md with content catalog
- Created log.md
- Added 9 concept pages: crdt-overlay, gossip-transport, iroh-transport, babel-routing, overlay-tun, hello-mesh, owner-claim, liveness-plane, meshd-binary
- Added 6 interop pages: interop-gossip-transport, interop-identity-sharing, interop-name-translation, interop-carrier-extension, interop-bulk-sync, interop-protocol-bridge
- Added 1 comparison: mesh-vs-reticulum
- Added 2 appendix pages: crypto-primitives, wire-formats
- Total: 18 files, ~1,000 lines

## 2025-11-05 (Rayfish)

- Added 1 concept page: rayfish-overview
- Added 5 interop pages: interop-rayfish-transport, interop-rayfish-addressing, interop-rayfish-firewall, interop-rayfish-dns, interop-rayfish-invite
- Added 1 comparison: mesh-vs-rayfish
- Updated index.md with Rayfish section
- Added 7 new files, ~29,000 lines

## 2025-11-05 (SDR/Radio)

- Added 1 research page: sdr-radio-integration
- Added Research section to index.md
- Total: 1 new file, ~15,000 lines

## 2025-11-05 (Baofeng/TNC/APRS)

|- Added 1 research page: baofeng-tnc-aprs-reticulum
|- Covers: Baofeng UV-5R, DireWolf TNC, APRS/AX.25, KISS protocol, Reticulum integration
|- Total: 1 new file, ~450 lines

## 2026-11-05 (Reticulum Codebase Grounding)

- Added 1 research page: reticulum-codebase-grounding
- Verified Reticulum source code: Identity.py, Link.py, Channel.py, Destination.py, Transport.py, Interface.py, Reticulum.py
- Grounded crypto primitives: 64-byte private key (32B X25519 + 32B Ed25519), SHA-256 truncated to 20B, HKDF with salt=identity.hash
- Grounded gossip transport: Channel.send() maps to broadcast(), Channel.get() is blocking (needs async bridge)
- Grounded packet wire format: 6 packet types, 84-byte header, AES-256-CBC encryption, token format = ephemeral_pub + ciphertext
- Grounded carrier interfaces: 10 Interface subclasses with MTU values and use cases
- Updated interop-gossip-transport.md with verified Channel/Link details
- Updated interop-identity-sharing.md with verified key layout details
- Updated crypto-primitives.md with verified HKDF, Token, Ratchets entries
- Updated wire-formats.md with verified packet types and announce structure
- Updated mesh-vs-reticulum.md crypto row with AES-256-CBC detail
- Updated interop-carrier-extension.md with full interface table
- Total: 1 new file, ~350 lines; 6 files patched

## 2026-11-05 (Rayfish Codebase Grounding)

- Added 2 new Rayfish pages: rayfish-codebase-grounding (451 lines), rayfish-identity-model (4,554 bytes)
- Verified Rayfish source code: identity.rs (160 lines), membership.rs (1,928 lines), transport.rs (57K lines), forward.rs (75K lines), firewall.rs (56K lines), dns/mod.rs (39K lines), invite.rs (35K lines), peers.rs (84K lines)
- **Key finding: Single iroh::SecretKey (Ed25519, 32 bytes)** — NOT dual X25519+Ed25519 like Reticulum. No separate key agreement key.
- **Key finding: Positional msgpack wire** — Field order in Member IS the wire format. New fields must be appended.
- **Key finding: One iroh endpoint per process** — All networks share a single iroh endpoint. Network separation is in-band via ALPN + 2-byte handles.
- **Key finding: IPv6-only overlay** — IPv4 (100.64.0.0/10 via FNV-1a) only for exit node egress. Overlay uses 200::/7 (blake3).
- **Key finding: Coordinator is mandatory** — Rayfish requires a coordinator (network key holder) for admission. Mesh's CRDT model is decentralized.
- **Key finding: Device cert model** — User signs device key. DeviceCert stored at ~/.config/rayfish/device_cert as msgpack.
- **Key finding: ExitFamilies enum** — Wire-optimized one-char serialization: ?, 4, 6, d, n
- **Key finding: Control plane DNS isolation** — Never asks own Magic DNS for control plane lookups. Falls back to public DNS.
- **Key finding: Forwarding architecture** — 3 concurrent tasks: TUN reader, per-peer readers, single TUN writer. FqCodel per peer.
- **Key finding: Firewall RuleOrigin model** — Local (hand-added, never touched), Network(net) (coordinator, replaced atomically), Ssh (auto-seeded, removed on ssh off).
- **Key finding: Invite format** — bs58(network_pubkey || coordinator || secret || checksum). Single-use, burned on redeem.
- **Key finding: Mesh vs Rayfish crypto** — Both use iroh::SecretKey (Ed25519, 32B). Mesh: blake3 for addressing. Rayfish: blake3(hex(identity)) for IPv6, FNV-1a for IPv4.
- Updated rayfish-overview.md: Added IPv6-only overlay note, single-key finding, msgpack wire note, coordinator requirement
- Updated interop-rayfish-transport.md: Line count corrected (57K, not ~900)
- Updated interop-rayfish-firewall.md: Added verified FirewallRule struct and RuleOrigin model
- Updated interop-rayfish-dns.md: Line count corrected (39K, not 45K), verified Magic DNS IP, TUN interception
- Updated interop-rayfish-invite.md: Line count corrected (35K, not 40K), added verified invite format
- Updated interop-rayfish-addressing.md: Added derive_ipv6() verified code, single-key finding
- Updated mesh-vs-rayfish.md comparison: Added positional msgpack, IPv6-only overlay, coordinator required rows
- Total: 2 new files, ~5,000 lines; 7 files patched; 1 comparison updated
