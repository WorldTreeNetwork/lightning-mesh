# source

# Addressing, Rayfish and this mesh

> Sources: tun/link.rs and Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish addressing](../raw/notes/2026-11-06-rayfish-addressing.md)
> Updated: 2026-11-06

Mesh already derives the node address from the node id.

- `backhaul_addr` is `10.254.<first 16 bits of blake3(node id)>`. The same id gets the same host on every boot. A collision is real in that 16-bit space. The claim record notices it, and the HLC loser moves with `backhaul_addr_salted`.
- `ula_addr` is the identity ULA. The prefix comes from a constant. The interface id is `blake3(node id)`. It is not hashed from the `10.254` address.
- The house prefix is `house_ula_prefix` of the guild id. That is a different prefix.
- Per-peer tunnel links are `/31`s in `10.255.0.0/16` from `pick_link_31`.

The client LAN is the part that is not derived. A node claims a `/24` from `10.42.0.0/16`. A different owner loses to the earlier hybrid logical clock.

Rayfish derives the overlay address itself: IPv6 in `200::/7` from blake3 of the endpoint id, stable across reboots. IPv4 in `100.64.0.0/10` is exit egress, and the overlay is IPv6-only. There is no second claimed client subnet.

So "adopt deterministic addressing" is not an open gap for the node. The node address is already a function of the identity. Copying Rayfish would mean changing the client `/24` and the ranges (`10.42`, `10.254`, the identity ULA), which this mesh is not doing.
