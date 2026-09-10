# add-coverage-sweep

> **ACTIVE BUILD**

Bead `mjolnir-mesh-6hn.6`. Human activated 2026-09-10.
Depends on coordinates, DreamBall payload, `/mesh` viz, and compass mark.

## Why

Marking a node is a point. The game is a region: walk, sample RSSI,
paint unknown → thin → covered, show completeness.

## What

- Geolocated coverage samples: compass GPS + heading + client-SSID RSSI
  (and/or node radio.json footprints joined by node id).
- Region completeness score on `/mesh`.
- First slice may replay a recorded walk on the simulated four-router
  fixture.

## Impact

- Capabilities: MODIFIED `mesh-coverage` (samples + game)
- ADRs: none
- Visualizer consumption: web3d-space `/mesh` (sibling change)

## User journey & surfaces

Duke walking a region with the Compass, watching `/mesh`.

1. **Working** — `/mesh` already animates radio links on metre fixtures.
2. **Empty** — no coverage cells; no completeness.
3. **Failed** — a walk that heard nothing leaves unknown, not covered.
4. **Off** — no GPS / no compass; fixture replay still works.

## Out of scope

- DGGS cell size bikeshed beyond one default the visualizer can draw
- Guild membership from coverage
- Live fleet RF heatmaps as a launch blocker (fixture replay is enough
  for the first slice)
