# add-client-network-name

> **ACTIVE BUILD**

Steer D4 v1 from identikey-core-trr.1. Orthogonal to guild keyspaces.
Radio join is not membership (D1).

## Why

Lightning Mesh already has a fleet-wide client SSID (`CLIENT_SSID`, applied
by `mjolnir-apply` → `setup-wireless.sh`). The guilds work needs that string
to stay a **network name**, not become a guild secret or a silent dual-write
with identikey-log. Factory boxes should ship ⚡ open without identikey.

## What

- Name the capability `client-network-name`.
- Contract: client SSID is a public radio name written only through
  `mjolnir-apply` / `setup-wireless.sh`. Copy says network name, not guild.
- Factory default SSID is ⚡ (U+26A1), encryption none. Existing fleet
  `CLIENT_SSID='Lightning Mesh'` stays until an operator apply.
- Association does not mint keyspace membership.

Capability: `client-network-name` (ADDED). `captive-portal` is not modified.

## Impact

- Capabilities: ADDED `client-network-name`
- ADRs: none

## User journey & surfaces

A fleet operator sets `CLIENT_SSID` in `deploy/openwrt/fleet-secrets/wireless.env`
(or `apply.env`) and runs `mjolnir-apply` with `RUN_WIRELESS=1`. Phones see
that name on the air. A visitor associates to factory ⚡ and reaches
`hello.mesh` without being a guild member. Empty: no `CLIENT_SSID` in the
staged env, `setup-wireless.sh` uses the factory default. Failed: apply
rolls back (`mjolnir-apply` health gate) and the previous SSID remains.
Off: `RUN_WIRELESS` not 1, SSID unchanged.

## Out of scope

- Guild join / identikey-log (`identikey-core-trr`, `add-guild-keyspaces`)
- Lightning Admin v2 verbs (`add-lightning-admin-guild`)
- `hello.mesh` / `bf7` UI
- PSK as guild secret (a PSK remains a radio filter only)
- Silently renaming the live Lightning Mesh fleet
- 802.11s backhaul `MESH_ID` (not the phone SSID)
