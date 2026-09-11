# steer add-coverage-survey

**When.** 2026-09-10
**Depth.** standard (architecture + protocol blast). First pass:
human said `awesome. let's activate all` — decide-for-me on intend
recommendations. Second pass: advise send-back (Fable 5.1) plus
human: routers are Wi-Fi only; compass has no GPS; phone selects a
nearby AP and enters GPS.

## Decided

- Roles: v1 surveyor is the **phone** (associated client AP + nearby
  radio strength + pick a node + enter GPS). Compass keeps magnetometer;
  compass GPS HAL is later, not a v1 dependency. AI Camera = cameras +
  LiDAR world model. No LiDAR on LilyGo T-RGB.
  Why: human 2026-09-10. Compass has no GPS today. Routers are Wi-Fi.
- Live store vs ball: last-known coords gossip on the mesh
  directory/CRDT; DreamBall is a signed snapshot + render recipe
  (decide-for-me)
  Why: directory.json is the existing hello/front-desk projection;
  radio.json v1 is a fixed RF contract; a ball cannot be the live store.
- Proximity to a **router**: Wi-Fi association + nearby client-BSSID
  RSSI, then the operator **selects** which node from that narrowed
  list and **enters GPS**. BLE/ESP-NOW is handheld-to-handheld only.
  Routers gain no NFC, BLE, or ESP-NOW silicon.
  Why: advise send-back (compass BLE disabled; mt76 fleet has no
  ESP-NOW peer). Human: phone sees connected AP and nearby strength.
- Stamp ingress: signed claim to **hello on the node's LAN gateway**,
  spooled to meshd. Hello verifies only; CRDT is the authority. Same
  shape as `POST /api/identity` / `POST /api/name-claim`.
  Why: advise send-back. Control API is loopback-only.
- Visualizer home: web3d-space `/mesh` (PlayCanvas) (decide-for-me)
  Why: `/mesh` already simulates GET /api/radio; browser-renderer ADR
  pins PlayCanvas; hello.mesh TopologyPanel is RF graph, not Earth.
- Game: region completeness from geolocated RSSI + node radio
  footprints; first slice may replay on the simulated walk (decide-for-me)
- Join key: `backhaul_addr` is the only key common to radio.json and
  directory.json today. [AUTO]
- Staleness: last-known is LWW by `stamped_at` + stamper; the viewer
  ages into thin/unknown. No tombstones in gossip. [AUTO]
- Privacy: directory holds full WGS84 last-known; DreamBall export is
  an explicit operator act. [AUTO]

## Skipped

- Exact DreamBall attribute schema (field names) — `add-dreamball-coverage`
- Coverage cell size / DGGS vs lat-lon heatmap — `add-coverage-sweep`
- Compass GPS HAL — later, on `add-compass-node-mark` after phone v1

## Feeds change

v1 stamp is a phone on the client SSID: hello.mesh on the associated
LAN gateway, a nearby-narrowed node list, operator pick, GPS from the
phone (geolocation or typed). Live store is the directory CRDT. Ball
is a snapshot. `/mesh` visualizes. Compass GPS and BLE-to-router are
refused for v1.
