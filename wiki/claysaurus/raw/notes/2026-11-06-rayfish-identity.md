# source

# Rayfish identity

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish identity](../raw/notes/2026-11-06-rayfish-identity.md)
> Updated: 2026-11-06

Rayfish keeps one `iroh::SecretKey`, 32 bytes, the same Ed25519 seed this mesh uses for a node id. It is not a Reticulum 64-byte blob. The notes say the file is the raw seed at `~/.config/rayfish/secret_key`. `load_existing` returns nothing when the file is absent. `load_or_create` mints a key only then.

A device cert is a user key's signature over a device public key. It is stored, checked on reload, and dropped by unpair. That is a user-and-device hierarchy. Mesh owner-claim is a different ceremony: one owner bound to one node by a device proof, a card HMAC, and a button window. It does not sign a second device key.

Mesh already stores an `iroh::SecretKey` and treats its public key as the node id. The device-cert shape is the part this mesh does not have.
