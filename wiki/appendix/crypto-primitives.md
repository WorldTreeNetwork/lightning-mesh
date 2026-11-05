---
title: Crypto Primitives Reference
created: 2025-11-05
updated: 2026-11-05
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
| Identity key | Ed25519 | `iroh::SecretKey` | 32-byte seed. The node id is the hex public key (`EndpointId`) |
| ULA and backhaul address | blake3 | `tun/link.rs` | `blake3` of the node id. This does not derive the node id |
| Serialization | postcard | `postcard` crate | Zero-copy, efficient binary format |
| Error types | thiserror | `thiserror` crate | Compile-time error conversion |

## Reticulum

| Primitive | Algorithm | Source | Notes |
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

- **Identity** -- Mesh node keys are Ed25519 `iroh::SecretKey` values (32 bytes). A Reticulum identity is 64 bytes: an X25519 key plus an Ed25519 key. The private keys are not interchangeable, and a signature from one stack is not a signature on the other.
- **Encryption** -- Mesh links use iroh's QUIC TLS. Reticulum packets use AES-256-CBC. The ciphers are different.
- **Hashing** -- Mesh uses blake3 for the ULA and the `10.254` address. Reticulum uses SHA-256, truncated for destination hashes. The address functions are not the same.
