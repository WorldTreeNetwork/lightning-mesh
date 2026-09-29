# add-sim-e2e-fade

> **ACTIVE BUILD**

Bead `mjolnir-mesh-0wgr.6`.

## Why

Need a gated fade of 802.11s. Live 2026-09-25: `vwifi-ctrl` distance
50000 does **not** drop `10.254` ping (partition via killing
vwifi-server does — that is health). Harness must FAIL until loss
model works.

## What

- `deploy/sim/e2e/run.sh fade`
- FAIL if overlay still up after fade
- Restore coordinates on EXIT

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`

## Out of scope

- Replacing `two_site_netns.rs`. Killing vwifi-server (health).
