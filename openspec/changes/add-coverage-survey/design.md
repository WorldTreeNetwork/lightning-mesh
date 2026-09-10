# Design: mesh-coverage

Cross-cutting contract. Implementation is sibling changes.

## Five calls

1. **Surveyor vs world-model camera.** The North Star Compass walks:
   GPS, magnetometer, tap-range proximity mark, RSSI of the client
   SSID. The AI Camera (All Systems Go) builds the world model:
   cameras + one Ethernet 3D lidar. Do not put lidar on the LilyGo
   T-RGB round display. Do not treat `mjolnir-mesh-e21` “north-star”
   roaming as this device.

2. **Live store vs DreamBall.** Last-known WGS84 lives as an additive
   optional field on the gossiped directory projection
   (`directory.json` / `GET /api/directory`), joined to radio.json by
   `node_id` / `mesh_mac` / `backhaul_addr`. Unmarked is absence, not
   origin. DreamBall is a signed snapshot of coverage plus a render
   recipe for the viewer data-input. It is not the live store.
   `radio.json` schema v1 stays RF-only. Coordinates do not go in
   UCI `mjolnir.radio`.

3. **Proximity mark.** “NFC-like” means tap-range presence: BLE or
   ESP-NOW already on the compass. The mesh accepts a stamp that
   names `node_id` + WGS84 + time + stamper. OpenWrt nodes do not
   gain NFC silicon.

4. **Visualizer home.** web3d-space `/mesh` (PlayCanvas, existing
   radio schema). Follow `browser-renderer`: this tree is not the
   raster host; `/mesh` already is. hello.mesh TopologyPanel stays
   the RF graph. myscape is not home.

5. **Game.** Region completeness from geolocated RSSI samples plus
   node radio footprints: unknown → thin → covered. First slice may
   replay a recorded walk on the simulated four-router fixture.

## Directory, not a new database

`DirectoryNode` / `DirectoryNeighbor` already grow additively (`name`,
`gateways`, `lost_names`). Coordinates follow that pattern. The CRDT
address book remains the coordination plane; hello reads the
projection, it does not become the writer.

## DreamBall

Coverage is a labeled attribute on a signed `ball/1` envelope, verified
only through `dreamball.wasm`. Do not add a fourth look/feel/act axis
in this architecture. Render recipe rides `look` and attributes.
Transmittable stays a store locator.

## Failure

Missing coordinate: omit the field. Corrupt ball: wasm failure, visible
in the data-input. Missed tap: write nothing. Missing PlayCanvas
adapter: visible error on `/mesh` (already).
