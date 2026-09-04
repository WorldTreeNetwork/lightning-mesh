# add-lightning-admin-network-name

> **ACTIVE BUILD**

Steer D4 v1. Radio slice of `mjolnir-mesh-st1.3`. Activated by `/run` to
ship the Lightning Admin network-name knob. Orthogonal to guild keyspaces.

## Why

The fleet already knows how to set the client SSID (`mjolnir-apply` with
`RUN_WIRELESS=1` → `setup-wireless.sh`). Lightning Admin is the native
operator desktop (`7oh`) but v1 is discovery-only: IPv6 neighbors, no
mutate path. Operators still hand-edit `wireless.env` and run
`update-fleet.sh --wireless`. That is the missing surface for the
guild-named mesh radio name.

## What

- Lightning Admin exposes a **network name** control (not "guild", not
  "SSID rename").
- Applying it stages `CLIENT_SSID` (and keeps open / `CLIENT_ENC=none`
  representable) and drives the existing fleet apply:
  `update-fleet.sh --wireless` → `install-node.sh --wireless` →
  `mjolnir-apply` `RUN_WIRELESS=1`.
- Same name on every reachable node (roam invariant). Unreachable nodes
  are skipped and reported, as today.
- Failed apply keeps the previous name (health-gated rollback already
  in `mjolnir-apply`).
- CLI apply remains valid. Admin is another writer of the same contract.

Capability: `client-network-name` (MODIFIED / ADDED Admin writer).

## Impact

- Capabilities: MODIFIED `client-network-name`
- ADRs: none

## User journey & surfaces

A fleet operator on a workstation that can already SSH the overlay
opens Lightning Admin (`admin/`). They type a network name, confirm,
and Apply. Working: phones see that name on every node the fleet
script reached. Empty: name field blank → Apply disabled. Failed:
`mjolnir-apply` rolls back; Admin shows the halt/skip list; previous
SSID remains on the air. Off: Admin not running; `update-fleet.sh
--wireless` still works.

## Out of scope

- Guild join / change / assign / nest — `mjolnir-mesh-st1.5` /
  `add-lightning-admin-guild` (blocked on identikey-core-trr.1)
- hello.mesh / `bf7` operator UI
- Per-node different SSIDs (breaks roam)
- WPS WAN-LAN SSH window — `mjolnir-mesh-m0d`
- babel proto 158 hardware gate — `mjolnir-mesh-1wy`
- Changing factory default ⚡ (already folded in st1.1)
- Installing workstation SSH keys on nodes
