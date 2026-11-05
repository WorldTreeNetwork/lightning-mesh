---
title: Shared Identity Across Stacks
created: 2025-11-05
updated: 2025-11-05
type: interop
tags: [interop, reticulum, identity-sharing]
sources: [crates/mjolnir-mesh/src/lib.rs, public/reticulum/RNS/Identity.py]
state: [FUTURE]
confidence: high
---

# Shared Identity Across Stacks

Both Lightning Mesh and Reticulum use Curve25519 for identity. A single Ed25519 keypair can serve as both a mesh node ID and a Reticulum Identity.

## Crypto Match

| Parameter | Lightning Mesh | Reticulum | Match |
|---|---|---|---|
| Curve | Curve25519 | Curve25519 | Yes |
| Signatures | Ed25519 (256-bit) | Ed25519 (256-bit) | Yes |
| Key Exchange | X25519 (ECDH) | X22519 (ECDH) | Yes |
| Key Size | 32 bytes | 32 bytes | Yes |

## Derivation

### Mesh to Reticulum

The mesh node SecretKey is already a Curve25519 key. To derive a Reticulum Identity:

1. Export the Ed25519 private key bytes from the mesh keypair
2. Create `RNS.Identity(private_key_bytes)`
3. The Reticulum destination hash is derived deterministically from the public key

**Verified from `Identity.py`:** Reticulum's `Identity.load_private_key()` expects exactly 64 bytes: `prv_bytes` (X25519, 32 bytes) + `sig_prv_bytes` (Ed25519, 32 bytes). The X25519 component is derived from the same seed as the Ed25519 key. If the mesh keypair is a pure Ed25519 key, the X25519 component must be derived from the same seed or generated separately.

### Reticulum to Mesh

1. Export the Reticulum Identity public key
2. Derive the mesh node ID: `blake3(reticulum_public_key_bytes)`
3. Use the same private key for mesh operations

**Verified from `Identity.py`:** The public key is `pub_bytes + sig_pub_bytes` (64 bytes total). The identity hash is `SHA-256(public_key)[:20]` (20 bytes). For mesh compatibility, use `blake3(public_key_bytes)` instead of the Reticulum hash.

## Benefits

- One key, two networks. No separate identity management.
- A node operator identity is consistent across both stacks.
- Cross-stack authentication becomes trivial verify the same signature.

## Risks

- Key compromise on one network compromises the other
- Different key rotation policies may be needed
- Reticulum Identity has a hierarchical name structure; mesh uses flat node IDs

## Implementation

This is a configuration-level change, not a code change. The key material is already in the mesh secret key store. A new config option `reticulum_identity: derive_from_mesh_key` would handle the mapping.
