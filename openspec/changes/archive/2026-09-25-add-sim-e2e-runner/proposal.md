# add-sim-e2e-runner

> **ACTIVE BUILD**

Bead `mjolnir-mesh-0wgr.1`. Human 2026-09-25: decide-for-me + activate.

## Why

The QEMU lab exists. Health, partition, and later suites are still
manual scripts. Without a runner, dual proto 158 stays a one-off log
line instead of a gated fail.

## What

- `deploy/sim/e2e/` runner: JSON lines, never `bd close`
- Health suite: SSH `.10`–`.14`, `sim-guest`, 802.11s ESTAB, `10.254`
  ping; kill vwifi → overlay dies, SSH lives; restore air

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

`deploy/sim/e2e/run.sh health` from the workstation. No new UI because
the surface is a CLI harness.

## Out of scope

- NAT / keep-IP / DNA / DHCP CRDT / fade / apply / stranger (`0wgr.2`–`.8`)
- Closing `5wc` / `wvg` / `sz9.1`
