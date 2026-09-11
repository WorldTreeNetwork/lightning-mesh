# add-identity-derived-ula

> **ACTIVE BUILD**

**Rigor:** architecture

Bead `mjolnir-mesh-v6.2`. Human activated 2026-09-11 (stone 2 of
`mjolnir-mesh-v6`). Does not reverse `bsa` (no IPv6 spine, no
v4-via-v6).

## Why

`10.254.<blake3(node_id)[0..2]>` is the same trick Tailscale uses, in
16 bits, so we needed `pt9` collision claims. A ULA has room to hash
the node id properly. OpenWrt already mints a *random* `ula_prefix`
per box (live pair: two different `/48`s) and babel filters it.
Identity-derived ULA is the parallel plane: never allocate, never a
lease CRDT.

## What

- Name the capability `identity-derived-ula`.
- Derive a mesh ULA `/48` from the domain-separated constant
  `mjolnir/mesh/ula/v0` (ASCII bytes, not the CRDT gossip topic) and
  a 64-bit IID from `blake3(node_id)` — RFC 4193 shape. Same spirit
  as `tun/link.rs` `backhaul_addr`, never coupled to the IPv4 host16.
- Assign that address as `/64` on **`br-mesh` only**, beside `10.254`.
  Deterministic across reboots. Re-added by the backhaul reconcile
  loop after wifi reload (`nodad`).
- Do **not** advertise it to iroh as a QUIC candidate (2026-06 ULA
  surface/DAD finding stands until re-measured).
- Do **not** RA it on the client AP (that's `v6.3`). Do **not**
  export it in babel IPv6 (that's `v6.4`).
- Stock OpenWrt `network.globals.ula_prefix` stays filtered
  (`redistribute local deny`).

## Impact

- Capabilities: ADDED `identity-derived-ula`
- ADRs: amend `ipv6-parallel-plane.md` with the exact derivation;
  `ARCHITECTURE.md` one sentence that nodes also have a hashed ULA
  next to `10.254`

## User journey & surfaces

An operator `GET`s `http://hello.mesh/api/node` (or `directory.json`)
and sees `ula` / `ula_prefix` for this node, stable across reboots,
computable from the node id without asking anyone. SSH to that ULA
works on-link once assigned (alongside LL first-contact from
`link-local-mgmt`). Empty: before assign, field omitted. Failed:
iroh still dials over `10.254`. Off: phones still DHCP v4.

No new UI because hello `/api/node` and Admin's ULA list already
exist; this makes the ULA *ours* instead of OpenWrt's random `/48`.

## Out of scope

- SLAAC RA / CAPPORT (`v6.3`, `aj1`)
- babel IPv6 FIB (`v6.4`)
- `.mesh` AAAA (`v6.5`)
- App TUN (`v6.6`)
- iroh ULA as QUIC candidate (re-measure bead, later)
- Replacing `10.254` or `pt9`
- `lan_tunnels`
