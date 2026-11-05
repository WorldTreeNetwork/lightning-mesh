---
title: Shared Identity Across Stacks
created: 2025-11-05
updated: 2026-11-05
type: interop
tags: [interop, reticulum, identity-sharing]
sources: [crates/mjolnir-mesh/src/lib.rs, public/reticulum/RNS/Identity.py]
state: [FUTURE]
confidence: high
---

# Shared Identity Across Stacks

A mesh node id is the hex Ed25519 public key of an `iroh::SecretKey` (32-byte seed). A Reticulum identity is a different object. This page is a proposal for a derivation. The tree does not implement it.

## Crypto Match

| Parameter | Lightning Mesh | Reticulum | Match |
|---|---|---|---|
| Signing key | Ed25519, 32-byte seed | Ed25519, 32 bytes inside a larger blob | Same algorithm, different container |
| Key agreement | iroh QUIC, not a second node key | X25519, 32 bytes | No |
| On-disk private key | 32-byte hex seed | 64 bytes: X25519 private plus Ed25519 private | No |
| Public id | 32-byte Ed25519 public key | 64-byte public blob, then SHA-256 truncated to 20 bytes | No |

## Derivation

### Mesh to Reticulum

`Identity.load_private_key()` expects 64 bytes: X25519 private key, then Ed25519 private key. A mesh secret is only the Ed25519 half. Producing a Reticulum identity means deriving or generating the X25519 half and storing the 64-byte blob. Passing the 32-byte mesh seed to `RNS.Identity` does not load a Reticulum identity.

### Reticulum to Mesh

The mesh node id is the Ed25519 public key, not `blake3` of a Reticulum public blob. Using a Reticulum signing key as an `iroh::SecretKey` is a new key ceremony, not a hash of the Reticulum identity. `blake3` in this repo derives the ULA and the `10.254` address from a node id that already exists.

## Risks

- A shared signing seed means compromise of one stack compromises the other
- The two formats rotate differently
- Reticulum destinations are hierarchical hashes. Mesh node ids are flat public keys

## What this is not

There is no config key that maps one identity onto the other. A future change would store the derived 64-byte Reticulum blob beside the mesh seed and would not treat the two private keys as the same bytes.
