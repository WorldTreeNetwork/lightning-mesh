---
title: Crypto Primitives Reference
created: 2025-11-05
updated: 2025-11-05
type: query
tags: [security, crypto, curve25519]
sources: [crates/mjolnir-mesh/, public/reticulum/]
state: [CURRENT]
confidence: high
---

# Crypto Primitives Reference

## Lightning Mesh

| Primitive | Algorithm | Source | Notes |
|---|---|---|---|
| Identity keys | Curve25519 | `iroh` crate | Ed25519 for signatures, X25519 for ECDH |
| Node ID derivation | blake3 | `blake3` crate | `blake3(node_id)` for overlay addressing |
| Serialization | postcard | `postcard` crate | Zero-copy, efficient binary format |
| Error types | thiserror | `thiserror` crate | Compile-time error conversion |

## Reticulum

|| Primitive | Algorithm | Source | Notes |
|---|---|---|---|
| Identity keys | Curve25519 | `RNS.Cryptography` | Ed25519 (32B) + X25519 (32B) = 64B total |
| Encryption | AES-256-CBC | `RNS.Cryptography.AES` | PKCS7 padding, ephemeral per-packet keys |
| Authentication | HMAC-SHA256 | `RNS.Cryptography.HMAC` | Per-packet message authentication |
| Key derivation | HKDF | `RNS.Cryptography.hkdf()` | ECDH shared key + salt (identity hash) + context (None) |
| Hashing | SHA-256 | `RNS.Cryptography.sha256()` | Full 32B hash; truncated to 20B for addresses |
| Token format | Fernet-like | `RNS.Cryptography.Token` | `ephemeral_pub + AES_ciphertext` |
| Padding | PKCS7 | `RNS.Cryptography.PKCS7` | Internal implementation |
| Pure-Python fallback | pure25519 | `RNS.Cryptography.pure25519` | When OpenSSL unavailable |
| Ratchets | X25519 | `RNS.Cryptography.X25519PrivateKey` | 30-day expiry, forward secrecy |

## Match Analysis

- **Curve25519** -- Direct match. Same curve, same key sizes. Private keys are interchangeable.
- **Signatures** -- Ed25519 in both stacks. A signature valid on one stack is valid on the other.
- **Key exchange** -- X25519 (Mesh) vs X22519 (Reticulum). X22519 is X25519 with clamping; the underlying curve is the same.
- **Encryption** -- Mesh uses Iroh's TLS 1.3 (ChaCha20-Poly1305 or AES-GCM). Reticulum uses AES-256-CBC. Different ciphers, same key material can derive both.
- **Hashing** -- Mesh uses blake3. Reticulum uses SHA-256/512. Different hash functions -- no direct compatibility for hash-based addressing.
