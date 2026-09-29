# add-sim-e2e-dhcp-crdt

> **ACTIVE BUILD**

Bead `mjolnir-mesh-0wgr.5`. Unblocked after `0wgr.3` folded.

## Why

wvg.4: every AP offers the first lease. Sim must show B ACK the same IP
or fail honestly if LeaseBook/hostsfile is empty (`join_island.run_dhcp`
unwired).

## What

- `deploy/sim/e2e/run.sh dhcp-crdt`
- After hop to B, DISCOVER/REQUEST yields the same IPv4
- B's `mjolnir-roam.conf` contains MAC→IP
- Does not close `wvg.4`

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

CLI harness. No new UI.

## Out of scope

- Closing `wvg.4`. Wiring `join_island.run_dhcp` (product).
