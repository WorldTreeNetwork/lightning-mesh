# steer add-coverage-survey

**When.** 2026-09-10
**Depth.** standard (architecture + protocol blast). Human said
`awesome. let's activate all` without menus — remaining forks are
decide-for-me on the intend recommendations.

## Decided

- Roles: Compass = GPS + magnetometer + proximity mark + RSSI walk;
  AI Camera = cameras + LiDAR world model (decide-for-me)
  Why: Northstar already has mag + BLE/ESP-NOW; LilyGo T-RGB has no
  lidar. AI Camera kit already has cameras + Ethernet 3D lidar.
- Live store vs ball: last-known coords gossip on the mesh
  directory/CRDT; DreamBall is a signed snapshot + render recipe
  (decide-for-me)
  Why: directory.json is the existing hello/front-desk projection;
  radio.json v1 is a fixed RF contract; a ball cannot be the live store.
- Proximity: tap-range BLE / ESP-NOW, not NFC on routers (decide-for-me)
  Why: compass already has BLE + ESP-NOW; OpenWrt fleet has no NFC.
- Visualizer home: web3d-space `/mesh` (PlayCanvas) (decide-for-me)
  Why: `/mesh` already simulates GET /api/radio; browser-renderer ADR
  pins PlayCanvas; hello.mesh TopologyPanel is RF graph, not Earth.
- Game: region completeness from geolocated RSSI + node radio
  footprints; first slice may replay on the simulated walk (decide-for-me)
  Why: details of render recipe stay with the ball; first paint can
  land without live GPS.

## Skipped

- Exact DreamBall attribute schema (field names) — `add-dreamball-coverage`
- Coverage cell size / DGGS vs lat-lon heatmap — `add-coverage-sweep`

## Feeds change

The ADR and `mesh-coverage` spec describe a directory-gossiped last-known
coordinate, a DreamBall snapshot for `/mesh`, a compass as the surveyor,
and an AI Camera as the world-model sensor. radio.json v1 stays RF-only.
NFC-on-router and GPS-on-every-AP are refused.
