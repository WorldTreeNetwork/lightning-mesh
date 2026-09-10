# add-compass-node-mark

> **ACTIVE BUILD**

Bead `mjolnir-mesh-6hn.5`. Human activated 2026-09-10.
Depends on `add-coverage-survey` and `add-node-coordinates`.

## Why

Last-known coordinates need a writer. The North Star Compass already
has a magnetometer and BLE/ESP-NOW. GPS HAL and a tap-range stamp do
not exist. Routers will not grow NFC.

## What

- Mesh ingest: a proximity stamp names `node_id` + WGS84 + time +
  stamper and writes the directory coordinate store.
- Compass firmware (VirtueInnova/northstar): GPS fix + heading from
  the existing magnetometer + one tap-range BLE/ESP-NOW gesture.
- Miss (no node in range) writes nothing.

## Impact

- Capabilities: MODIFIED `mesh-coverage` (stamp ingest)
- ADRs: none
- Firmware landing: `/Users/dukejones/work/VirtueInnova/northstar`

## User journey & surfaces

Duke standing next to a live node with the Compass.

1. **Working** — compass shows magnetic heading today.
2. **Empty** — no GPS; no gesture that writes a mesh coordinate.
3. **Failed** — no node in range → no write; no GPS fix → no write.
4. **Off** — directory store not yet deployed on the node.

The surface is the physical compass plus the existing directory
projection. No new hello.mesh panel.

## Out of scope

- NFC silicon on OpenWrt
- LiDAR / cameras on the T-RGB
- Sweep sampling / game score — `add-coverage-sweep`
- DreamBall export — `add-dreamball-coverage`
