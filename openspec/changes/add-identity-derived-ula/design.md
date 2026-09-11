# Design: identity-derived ULA

Advise send-back 2026-09-11 (`fable-5.1-arch-review`) pinned the
following. Product code still waits on a second accept.

## Derivation

RFC 4193: `fd` + 40-bit Global ID + 16-bit Subnet ID + 64-bit Interface ID.

Pure function next to `backhaul_addr` in `tun/link.rs`. Never coupled
to the IPv4 host16 (`pt9` may move `10.254`; the ULA hashes `node_id`
alone and does not move).

- **Global ID** = first 40 bits of `blake3(b"mjolnir/mesh/ula/v0")`.
  Domain-separated from the CRDT gossip topic (`mjolnir/mesh/crdt/v0`).
  Bumping gossip does not renumber ULAs. Exact input is the 20 ASCII
  bytes `mjolnir/mesh/ula/v0` — not a TopicId, not a PSK. A golden
  vector test locks one `(constant, node_id)` → address. Federation
  (`yau`) may later introduce a per-mesh /48; not this change. The
  `/48` is therefore a compiled constant in practice, so an app can
  compute a node's ULA from node id alone.
- **Subnet ID** = 0 for the node overlay (client `/64`s are `v6.3`).
- **Interface ID** = first 64 bits of `blake3(node_id)` as opaque bits
  (RFC 7136; no u/l games). Clamp IID 0 (subnet-router anycast) the
  same way `backhaul_addr` clamps `.0.0`.

Assign on `br-mesh` as **`/64`** (subnet id 0 + 64-bit IID) so every
node's ULA is on-link on the backhaul bridge. That is what makes
`ssh root@<ula>` work without babel v6.

## Where it lives

**`br-mesh` only** in this change. Do not assign on `mjolnir0`
(connected `/64` on the TUN races the `br-mesh` on-link route and
black-holes until `v6.4` has a v6 FIB). Not on `br-lan` until RA
(`v6.3`).

## DAD and reconcile

netifd wifi reload flushes `br-mesh` addresses. `reconcile_backhaul_addr`
already re-adds `10.254`; it SHALL also re-add the ULA. Kernel `fe80`
comes back on its own; a manually added ULA does not.

Add the address **`nodad`**. Binding a ULA already tripped IPv6 DAD for
iroh (2026-06); we are not offering it as a QUIC bind, but a 1 s
tentative window on `br-mesh` after every wifi reload is still a
management-plane hole. `nodad` matches "this address is a hash, not a
SLAAC guess."

## Babel containment

`redistribute local deny` is family-agnostic. The `in`/`out` deny
lines today are v4-only. Both babeld renders (br-mesh / overlay)
SHALL also `in ip fc00::/7 deny` and `out ip fc00::/7 deny` so v6.4
has to opt in. Stock OpenWrt `ula_prefix` stays filtered.

## iroh non-candidate

Do not rely on iroh 1.0 happening not to surface ULA. Filter
explicitly, and expose a runtime observable (status / `/api/node`
candidates, or a fleet health-gate grep that no peer candidate
starts with `fd`).

## What we refuse

- Feeding the ULA to iroh's bind set.
- Assigning it on `mjolnir0` or `br-lan` in this change.
- Replacing OpenWrt's random LAN `ula_prefix`.
- v4-via-v6 next hops.
- Hashing the CRDT topic or the IPv4 host16 into the ULA.
