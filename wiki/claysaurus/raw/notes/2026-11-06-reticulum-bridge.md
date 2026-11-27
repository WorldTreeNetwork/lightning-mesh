# source

# A mesh to Reticulum gateway

> Sources: Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum bridge](../raw/notes/2026-11-06-reticulum-bridge.md)
> Updated: 2026-11-06

The draft describes a daemon that would hide which network a `foo.mesh` peer is on. There is no such daemon.

The two stacks do not join the same way. A mesh router is owned by the owner-claim ceremony. A Reticulum link, in the notes, is an explicit handshake. The node id is an Ed25519 public key. A Reticulum destination is a truncated hash. iroh gossip delivers one postcard message. A Reticulum channel, in the notes, is a reliable stream. None of that is translated in this tree.

The draft called this a later phase after a transport swap. The transport swap is also not accepted. This page stays a note.
