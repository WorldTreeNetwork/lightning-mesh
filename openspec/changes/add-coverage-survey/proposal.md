# add-coverage-survey

> **ACTIVE BUILD**

Bead `mjolnir-mesh-6hn.1`. Human activated 2026-09-10 (`activate all`).
Steer: decide-for-me on the intend recommendations (`steer.md`).

**Rigor:** architecture

## Why

Lightning Mesh knows RF (radio.json v1) and overlay identity (directory.json)
but not Earth. The coverage-survey game needs last-known node coordinates,
a signed DreamBall snapshot for the viewer, a handheld mark, and a
visualizer — four repos — or later nodes will fight over where GPS lives
and which device is the surveyor.

## What

- Name capability `mesh-coverage`.
- Record five architecture calls (roles, live store vs ball, proximity,
  visualizer home, game).
- Amend `ARCHITECTURE.md` with those calls.
- Do not implement directory fields, DreamBall payload, `/mesh` heatmap,
  compass firmware, or the sweep game in this change.

## Impact

- Capabilities: ADDED `mesh-coverage`
- ADRs: will amend `ARCHITECTURE.md`
- `radio.json` v1 unchanged
- hello.mesh TopologyPanel unchanged
- `mjolnir-settings` / UCI `mjolnir.radio` unchanged

## User journey & surfaces

Duke (and later a walker with a North Star Compass), from the existing
web3d-space `/mesh` simulated radio world.

1. **Working** — `/mesh` already draws the four-router fixture with
   hardcoded metre positions and radio.json-shaped links.
2. **Empty** — no last-known WGS84 on a node; no DreamBall coverage
   payload; no proximity stamp; no coverage paint (covered / thin /
   unknown).
3. **Failed** — a missing coordinate must be absence, not `(0,0)`; a
   corrupt ball must fail visibly in the viewer data-input.
4. **Off** — this change writes the contract only. Coordinates are
   `add-node-coordinates`. Ball is `add-dreamball-coverage`. `/mesh`
   paint is `add-coverage-world-viz`. Stamp is `add-compass-node-mark`.
   Sweep is `add-coverage-sweep`. World model overlay is
   `add-survey-world-model`.

No new UI because `/mesh` already is the entry; this node is the
contract later nodes implement.

## Out of scope

- Directory lat/lon fields — `add-node-coordinates` (`mjolnir-mesh-6hn.2`)
- DreamBall coverage attribute — `add-dreamball-coverage` (`mjolnir-mesh-6hn.3`)
- `/mesh` coverage paint — `add-coverage-world-viz` (`mjolnir-mesh-6hn.4`)
- Compass GPS HAL + tap — `add-compass-node-mark` (`mjolnir-mesh-6hn.5`)
- Sweep game score — `add-coverage-sweep` (`mjolnir-mesh-6hn.6`)
- Cameras/LiDAR world overlay — `add-survey-world-model` (`mjolnir-mesh-6hn.7`)
- GPS hardware on OpenWrt nodes
- NFC silicon on routers
- Replacing hello.mesh TopologyPanel
- In-browser NeRF train
- Coverage as guild membership
- `mjolnir-mesh-e21` same-SSID “north-star” roaming (different name)
