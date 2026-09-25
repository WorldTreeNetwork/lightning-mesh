# add-sim-roam-keep-ip

> **ACTIVE BUILD**

Bead `mjolnir-mesh-sim.7`. Human activated 2026-09-18 (`activate all`).
**Rigor:** instrument. Advise (other-family reader) before act.

## Why

`5wc` is a walking-phone test. The lab's first instrument is a Linux STA
hop A→B that records keep-IP, proto 158, LeaseBook, and egress gap.
Sim green does not close `5wc` / `wvg` / `sz9.1`.

## What

- OpenWrt STA-only guest (same image)
- Associate A, hop to B (vwifi-ctrl and/or wpa_cli roam)
- Record IP, `/32` on B only, LeaseEntry, ping gap
- Must not `bd close` metal roam beads

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

No new UI because the surface is a CLI harness log under `deploy/sim/`.

## Out of scope

- Closing `5wc` / `wvg` / `sz9.1`
- iPhone DNAv4 (`sim.9` / `sz9.1`)
- Call-survive (`wvg.1`)
