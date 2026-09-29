# add-sim-e2e-keep-ip

> **ACTIVE BUILD**

Bead `mjolnir-mesh-0wgr.3`. Human 2026-09-25: decide-for-me + activate.
**Rigor:** instrument. Advise before act.

## Why

`roam-keep-ip.sh` already showed keep-IPv4 and dual proto 158. That
must be a gated fail, not an xfail and not a metal close.

## What

- Wrap the hop harness as `deploy/sim/e2e/run.sh keep-ip`
- Dual proto 158 (present on A and B) is FAIL
- Never `bd close` `5wc` / `wvg` / `sz9.1`

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

No new UI because the surface is the CLI harness log.

## Out of scope

- Fixing meshd dual `/32` (product, `sz9`)
- Closing `5wc`
- iPhone DNAv4 (`0wgr.4` / `sz9.1`)
