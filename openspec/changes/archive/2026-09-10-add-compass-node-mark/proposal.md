# add-compass-node-mark

> **ACTIVE BUILD**

Bead `mjolnir-mesh-6hn.5`. Human activated 2026-09-10.
Depends on `add-coverage-survey` and `add-node-coordinates`.
Steer (2026-09-10, after advise send-back): v1 writer is a **phone**;
compass has no GPS yet.

## Why

Last-known coordinates need a writer. Routers are Wi-Fi. The compass
has a magnetometer and no GPS. A phone on the client SSID can see the
associated AP, radio strength of nearby nodes, narrow the list, pick
a node, and enter GPS.

## What

- hello.mesh stamp UI on the associated node's LAN gateway: nearby-
  narrowed node list (associated node default), operator pick, GPS
  from the phone (geolocation or typed).
- Signed stamp `POST` to hello; hello verifies and spools; meshd
  ingests into the directory CRDT. Same shape as identity / name-claim.
- Empty list, cancel, or no GPS and nothing typed: write nothing.
- Compass GPS HAL is later, not this slice.

## Impact

- Capabilities: MODIFIED `mesh-coverage` (stamp ingest + hello UI)
- ADRs: none (follows `add-coverage-survey`)

## User journey & surfaces

Duke standing next to a live node with a phone on Lightning Mesh.

1. **Working** — phone is associated; hello.mesh loads on
   `http://10.42.<x>.1`; directory lists self + neighbors.
2. **Empty** — no stamp form; no last-known coords.
3. **Failed** — no nearby nodes, cancel, or missing GPS with nothing
   typed → no write. Bad signature → hello 400, no spool.
4. **Off** — not on the client SSID; overlay/hello unreachable from
   the house WAN without the WPS window.

## Out of scope

- Compass GPS HAL (later)
- NFC / BLE / ESP-NOW silicon on OpenWrt
- LiDAR / cameras on the T-RGB
- Sweep sampling / game score — `add-coverage-sweep`
- DreamBall export — `add-dreamball-coverage`
- Replacing TopologyPanel
