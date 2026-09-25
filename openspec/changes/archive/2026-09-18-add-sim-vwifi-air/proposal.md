# add-sim-vwifi-air

> **ACTIVE BUILD**

Bead `mjolnir-mesh-sim.4`. Human activated 2026-09-18 (`activate all`).
Advise not required (change rigor). Act after `sim.2` image and `sim.3` domains.

## Why

802.11s and the client AP need a medium between QEMU guests. vwifi + hwsim
is the pinned air.

## What

- vwifi-server on the host; vwifi-client in node guests
- Two hwsim radios; 802.11s plink; client AP; `vwifi-ctrl` distance
- Control on mgmt TCP, not wifi

## Impact

- Capabilities: MODIFIED `mesh-sim-lab`
- ADRs: none

## User journey & surfaces

No new UI because the surfaces are `vwifi-server`, `iw`, and `deploy/sim/`.

## Out of scope

- install-node profile (`sim.5`), roam harness (`sim.7`)
