# add-link-local-mgmt

> **ACTIVE BUILD**

Bead `mjolnir-mesh-v6.1`. Human activated 2026-09-11 (stone 1 of
`mjolnir-mesh-v6`: LL as a management address).

## Why

When overlay `10.254` and WAN IPv4 are missing, operators already SSH
and discover boxes on `fe80::/10`. That path is field-proven (Admin
scan, AP3000 notes) but unpublished: `directory.json` / `GET /api/node`
only expose `backhaul_addr`. First contact should not require DHCP, a
WAN lease, or the overlay.

## What

- Name the capability `link-local-mgmt`.
- meshd collects this node's kernel Unique Local is out of scope here
  (that's `v6.2`); collect **unicast link-local** on `br-lan` and
  `br-mesh` and project them onto `DirectoryNode` (additive JSON,
  omitted when empty).
- Do **not** publish `mjolnir0`'s derived `overlay_link_local` as the
  management address — that `fe80` is babel-on-TUN, not SSH from the
  SSID.
- hello `GET /api/node` and `GET /api/directory` keep serving the node
  object; new fields ride along.
- Lightning Admin's existing `scan_link_local` list stays the operator
  surface (scoped copy already works).
- Overlay SSH at `10.254` remains the mesh-wide management plane.

## Impact

- Capabilities: ADDED `link-local-mgmt`
- ADRs: amend `ARCHITECTURE.md` one sentence — LL is first contact;
  overlay remains mesh-wide SSH. Does not reverse `bsa`.

## User journey & surfaces

An operator on the client SSID (or the node's LAN) opens Lightning
Admin, scans, and sees `fe80` rows (already). After this change they
can also `GET http://hello.mesh/api/node` (or the directory) and read
this box's `br-lan` / `br-mesh` link-locals, then
`ssh -o BatchMode=yes root@fe80::…%wlan0` with an authorized key.

Working: associated to the SSID or on-link with `br-lan`/`br-mesh`,
dropbear accepts the key, no IPv4 lease required. Empty: iface has no
LL (omit the field). Failed: overlay down, WAN closed — LL still
works. Off: operator not on that L2; LL cannot leave the link (use
`10.254` once on the mesh).

No new UI because Admin already lists link-local; hello `/api/node`
already exists.

## Out of scope

- Identity-derived ULA (`mjolnir-mesh-v6.2` / later change)
- SLAAC RA / CAPPORT on RA (`v6.3`, `aj1`)
- babel IPv6 FIB (`v6.4`)
- App-side TUN (`v6.6`)
- Re-enabling `lan_tunnels`
- Replacing `10.254` as iroh underlay or mesh-wide SSH
- Publishing stock OpenWrt random `ula_prefix`
