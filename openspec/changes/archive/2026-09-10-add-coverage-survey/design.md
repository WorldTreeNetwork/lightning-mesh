# Design: mesh-coverage

Cross-cutting contract. Implementation is sibling changes.

## Five calls

1. **Surveyor vs world-model camera.** v1 surveyor is a **phone** on
   the client SSID: it sees the associated AP, radio strength of
   nearby nodes, narrows the list, the operator picks a node and
   enters GPS (browser geolocation or typed). The North Star Compass
   already has a magnetometer; it has **no GPS today** — compass GPS
   HAL is later, not this architecture's v1 path. The AI Camera
   (All Systems Go) builds the world model: cameras + one Ethernet
   3D lidar. Do not put lidar on the LilyGo T-RGB. Do not treat
   `mjolnir-mesh-e21` “north-star” roaming as this device.

2. **Live store vs DreamBall.** Last-known WGS84 lives as an additive
   optional field on the gossiped directory projection
   (`directory.json` / `GET /api/directory`). Join to `radio.json` is
   **`backhaul_addr`** (the only key both documents have today).
   Unmarked is absence, not origin. Last-known is LWW by `stamped_at`
   + stamper; the viewer ages stale stamps into thin/unknown; gossip
   does not grow tombstones. DreamBall is a signed snapshot of
   coverage plus a render recipe for the viewer data-input. It is
   not the live store. Exporting a ball is an explicit operator act.
   `radio.json` schema v1 stays RF-only. Coordinates do not go in
   UCI `mjolnir.radio`.

3. **Proximity mark.** Routers are Wi-Fi. Proximity to a router is
   **association to that node's client AP** plus **nearby client-BSSID
   RSSI** to rank the rest. The operator **selects** which node from
   that narrowed list and **enters GPS**. There is no automatic tap
   onto a box. BLE/ESP-NOW is handheld-to-handheld only (compass BLE
   is disabled against the RGB LCD DMA; the mt76 fleet has no ESP-NOW
   peer and no `hci0`). OpenWrt nodes gain no NFC, BLE, or ESP-NOW
   silicon.

4. **Visualizer home.** web3d-space `/mesh` (PlayCanvas, existing
   radio schema). hello.mesh TopologyPanel stays the RF graph. The
   **stamp UI** is hello.mesh on the associated node's LAN gateway
   (select node + GPS) — that is not TopologyPanel and not `/mesh`.
   myscape is not home.

5. **Game.** Region completeness from geolocated RSSI samples plus
   node radio footprints: unknown → thin → covered. First slice may
   replay a recorded walk on the simulated four-router fixture.

## Directory, not a new database

`DirectoryNode` / `DirectoryNeighbor` already grow additively (`name`,
`gateways`, `lost_names`). Coordinates follow that pattern. The CRDT
address book is the authority.

Hello is a **verifying spool**, never the authority: a coordinate
stamp is a signed claim `POST`ed to hello on the node's LAN gateway
(`10.42.<x>.1`, same pattern as `POST /api/identity` and
`POST /api/name-claim`), written to `--spool-dir`, swept by meshd
into the CRDT. Hello holds no key; it only verifies. The meshd
control API stays loopback (`127.0.0.1:5380`) and is not the
handheld door.

## DreamBall

Coverage is a labeled attribute on a signed `ball/1` envelope, verified
only through `dreamball.wasm`. Do not add a fourth look/feel/act axis
in this architecture. Render recipe rides `look` and attributes.
Transmittable stays a store locator. Minting/exporting a coverage ball
is an explicit operator act, not a side effect of a stamp.

## Failure

Missing coordinate: omit the field. Corrupt ball: wasm failure, visible
in the data-input. Empty nearby list / declined pick: write nothing.
No GPS on the phone and nothing typed: write nothing. Missing PlayCanvas
adapter: visible error on `/mesh` (already).
