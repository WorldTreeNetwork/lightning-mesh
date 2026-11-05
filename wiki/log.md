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
