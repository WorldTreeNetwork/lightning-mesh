# source

# Rayfish grounding

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish grounding](../raw/notes/2026-11-06-rayfish-grounding.md)
> Updated: 2026-11-06

The draft named `identity.rs`, `membership.rs`, `transport.rs`, `firewall.rs`, `forward.rs`, `dns/mod.rs`, `invite.rs`, and `peers.rs` inside a Rayfish tree that is not this repository. Nothing here re-opened that tree. Where the draft's own numbers disagree, this page drops them.

What the notes agree on:

- One `iroh::SecretKey` for the endpoint.
- `derive_ipv6` hashes the identity with blake3 into `200::/7`. IPv4 `100.64.0.0/10` uses FNV-1a and is not the overlay.
- Membership is a roster plus a signed GroupBlob. A coordinator admits.
- The firewall sees TUN packets. It does not filter the QUIC control streams.
- Invites are single-use codes checked by that coordinator.

What this mesh does, checked here:

- `backhaul_addr` and `ula_addr` in `crates/mjolnir-mesh/src/tun/link.rs` derive the node address from the node id.
- A client subnet is a `SubnetClaim` on `10.42.0.0/16`, not a hash of the node id.
- There is no Rayfish conntrack module on `mjolnir0`.
- `is_unique_local` refuses Unique Local addresses as iroh candidates. That is not Rayfish's CGNAT filter.
