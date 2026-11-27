# source

# Two identity blobs

> Sources: this tree and Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum identity](../raw/notes/2026-11-06-reticulum-identity.md)
> Updated: 2026-11-06

A mesh node id is the hex Ed25519 public key of an `iroh::SecretKey`, a 32-byte seed. A Reticulum identity is a different object. This page is a note about a derivation that does not exist.

| | Lightning Mesh | Reticulum |
|---|---|---|
| Signing | Ed25519, 32-byte seed | Ed25519 bytes inside a larger blob |
| Key agreement | iroh QUIC. Not a second node key | X25519, 32 bytes |
| Private key on disk | 32-byte seed | 64 bytes: X25519 private, then Ed25519 private |
| Public id | 32-byte Ed25519 public key | 64-byte public blob, then SHA-256 truncated |

`Identity.load_private_key()` expects the 64-byte blob. The 32-byte mesh seed does not load. The other direction is also a new key, not a hash: the mesh node id is the Ed25519 public key itself. `blake3` in this repo derives the ULA and the `10.254` host from a node id that already exists. It does not derive that id from a Reticulum public blob.

There is no config key that maps one identity onto the other. A shared signing seed would mean compromise of one stack compromises the other. That is a reason not to do it casually, not a design that has been accepted.
