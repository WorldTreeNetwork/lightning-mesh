# add-mjolnir-settings-store

> **ACTIVE BUILD**

Beads `mjolnir-mesh-849.1` (decision, activated) and `mjolnir-mesh-849.2`.
Human: activate and run .1 and .2.

## Why

WPS WAN-LAN SSH timeout is a magic 900 in `mjolnir-wan-admin`. Client
SSID / encryption / password are staged via `wireless.env` and then live
only in UCI `wireless`. A later operator UI needs one Lightning store to
edit; OpenWrt should learn those knobs through `mjolnir-apply`.

## What

- Capability `mjolnir-settings`.
- `/etc/config/mjolnir` sections `radio` and `wan_admin` are the live store.
- Apply ensures the sections (defaults: ⚡ / `none` / timeout 900). A staged
  `wireless.env` **writes the store**, then apply **projects** store → UCI
  wireless via `setup-wireless.sh`.
- `mjolnir-wan-admin` reads `mjolnir.wan_admin.timeout` (fallback 900).
- LuCI-edited `wireless` is not canonical; the next apply overwrites it
  from the store.

## Impact

- Capabilities: ADDED `mjolnir-settings`
- ADRs: none this wave (shape note for ARCHITECTURE.md on fold)
- `client-network-name` unchanged (network name, apply is the radio write)
- `wan-lan-admin` timeout becomes a setting (still prefix-boxed, still WPS)

## User journey & surfaces

An operator (or `wireless.env` on install) sets `mjolnir.radio.*` and
`mjolnir.wan_admin.timeout`. `mjolnir-apply` writes OpenWrt wireless.
WPS press uses the stored timeout. Empty: unset store → factory ⚡ / none /
900s. Failed: apply health-gate rollback. Off: `RUN_WIRELESS` not 1, radio
UCI unchanged; timeout still readable by wan-admin.

No new UI because 849.3 (GL.iNet-style operator UI) is the later writer.
This change is the store + apply projector.

## Out of scope

- On-box operator UI / LuCI kickover (`mjolnir-mesh-849.3`)
- Lightning Admin SSID form (`st1.3`) — will write this store later
- Backhaul MESH_ID / MESH_KEY / channels in the store
- Fold of `add-wps-wan-admin`
- ARCHITECTURE.md prose (needs sign-off)
