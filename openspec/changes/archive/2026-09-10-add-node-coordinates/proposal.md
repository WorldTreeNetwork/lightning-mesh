# add-node-coordinates

> **ACTIVE BUILD**

Bead `mjolnir-mesh-6hn.2`. Human activated 2026-09-10.
Depends on `add-coverage-survey` (advise + ADR).

## Why

Directory.json names nodes, subnets, and backhaul addrs. Nothing on
Earth. The compass stamp and the `/mesh` visualizer need a last-known
coordinate they can join to radio.json without breaking schema v1.

## What

- Capability `mesh-coverage` (MODIFIED by this change once survey folds;
  ADDED here if this lands first as additive directory fields).
- Additive optional WGS84 + stamp metadata on `DirectoryNode` and
  `DirectoryNeighbor`.
- Gossip the stamp on the existing CRDT address-book path. Hello reads
  the projection; it does not become the writer.
- Unmarked = field omitted. Never a fake origin.

## Impact

- Capabilities: MODIFIED `mesh-coverage` (coordinates)
- ADRs: none (follows `add-coverage-survey`)
- `radio.json` v1 unchanged

## User journey & surfaces

An operator (or later the compass) stamps a node. hello.mesh
`GET /api/directory` and the web3d-space `/mesh` data-input can read
the coordinate.

1. **Working** — directory already lists self + neighbors with `node_id`,
   `backhaul_addr`, optional `name`.
2. **Empty** — no lat/lon on those entries.
3. **Failed** — a node never stamped omits the field; consumers must not
   invent `(0,0)`.
4. **Off** — stamp ingest is `add-compass-node-mark`. This change is the
   store + projection.

No new UI because `GET /api/directory` already is the surface.

## Out of scope

- Compass GPS HAL / BLE tap — `add-compass-node-mark`
- DreamBall snapshot — `add-dreamball-coverage`
- `/mesh` paint — `add-coverage-world-viz`
- GPS hardware on OpenWrt
- Breaking `radio.json` v1
