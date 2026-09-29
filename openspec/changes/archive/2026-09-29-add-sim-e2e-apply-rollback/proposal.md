# add-sim-e2e-apply-rollback

> **ACTIVE BUILD**

Bead `mjolnir-mesh-0wgr.7`. Unblocked after `0wgr.1`. Intend: no extra
human-gate (epic activated 2026-09-25 decide-for-me).

## Why

The lab can break UCI without touching metal. Apply's contract is
snapshot → apply → health gate → restore. That path is still a
manual story.

## What

- `deploy/sim/e2e/run.sh apply-rollback`
- On a marked sim guest: `mjolnir-apply` snapshots, a staged UCI
  mutation fails closed before commit, snapshot is restored
- No `wifi reload` (hostapd owns the sim client AP)
- Result is `ROLLED_BACK`; overlay `10.254` still works
- Never `bd close`; metal apply stays `lpv` / `z3th`

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

No new UI because the surface is the CLI harness
`deploy/sim/e2e/run.sh apply-rollback`.

## Out of scope

- Metal apply / radio qualification (`mjolnir-mesh-lpv`, `z3th`)
- Stranger node (`0wgr.8` waits `fyby.2`)
- Closing `5wc` / `wvg` / `sz9.1`
- Replacing `two_site_netns.rs`
