# add-sim-node-profile

> **ACTIVE BUILD**

Bead `mjolnir-mesh-sim.5`. Human activated 2026-09-18 (`activate all`).

## Why

`install-node.sh` / `mjolnir-apply` must run on sim guests with an hwsim
wireless profile. The profile must refuse unmarked metal (positive
sim-guest marker).

## What

- install-node against sim SSH
- hwsim 2g+5g so setup-wireless.sh discovers radios
- Guard: no `/etc/mjolnir/sim-guest` → abort before wireless write

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

No new UI because the surfaces are `install-node.sh` and `mjolnir-apply`.

## Out of scope

- NAT fixture (`sim.6`), roam harness (`sim.7`)
