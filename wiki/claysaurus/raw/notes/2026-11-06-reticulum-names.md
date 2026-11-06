# source

# Name translation

> Sources: Clay's Reticulum notes, 2026-11-06
> Raw: [Reticulum names](../raw/notes/2026-11-06-reticulum-names.md)
> Updated: 2026-11-06

The draft proposes a daemon that publishes a `.mesh` name as a Reticulum destination and the reverse. Nothing in this tree does that. There is no `mesh-rns-bridge`.

Mesh names are the `.mesh` zone on the node: leased names, the directory, and hello.mesh on that node's LAN gateway. A Reticulum destination, in the notes, is a truncated SHA-256 of a name and an identity hash. Those are different lookup keys. A table that pairs them would be new state, not a row the CRDT books already have.

The draft's example labels (`myapp.mesh` mapping to `owner.myapp.mesh`) are illustrations. They are not records in the fleet.
