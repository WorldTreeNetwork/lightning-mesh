## ADDED Requirements

### Requirement: Last-known coordinates live on the directory

Last-known node coordinates SHALL be additive optional fields on the
gossiped directory projection (`directory.json` / `GET /api/directory`).
They SHALL be joinable to `radio.json` by `node_id`, `mesh_mac`, or
`backhaul_addr`. `radio.json` schema v1 SHALL remain RF-only. Coordinates
SHALL NOT live in UCI `mjolnir.radio` or in a central database.

#### Scenario: Marked node

- GIVEN a node whose directory entry has a last-known WGS84 stamp
- WHEN a consumer reads `GET /api/directory`
- THEN that node's coordinate is present and attributable to a stamper
  and time

#### Scenario: Unmarked node

- GIVEN a node that has never been stamped
- WHEN a consumer reads its directory entry
- THEN the coordinate field is absent, not `(0,0)` or a metre-space fixture

#### Scenario: Radio snapshot stays RF

- GIVEN `GET /api/radio` schema v1
- WHEN this capability is folded
- THEN the radio document still has no lat/lon fields

### Requirement: DreamBall is a snapshot, not the live store

A signed DreamBall MAY carry a coverage document and a render recipe for
a viewer data-input. The live last-known coordinate SHALL remain the
directory projection. The ball SHALL be verified only through
`dreamball.wasm` (`verifyBall` then `parseBall`). Coverage SHALL be a
labeled attribute on a `ball/1` envelope, not a fourth look/feel/act axis.

#### Scenario: Viewer data-input

- GIVEN a valid signed `.ball` whose attributes include coverage
- WHEN the `/mesh` data-input loads it
- THEN wasm verify+parse succeeds and the enclosed nodes are available
  to the visualizer

#### Scenario: Corrupt ball

- GIVEN bytes that fail `verifyBall` or `parseBall`
- WHEN the data-input handles them
- THEN the failure is the wasm reason, no DreamBall is invented, and
  the surface does not hang

### Requirement: Compass is the surveyor; AI Camera is the world-model sensor

The North Star Compass SHALL be the handheld surveyor: GPS, magnetometer,
tap-range proximity mark, and RSSI walk. World-model capture (cameras and
LiDAR) SHALL be the AI Camera path. The LilyGo T-RGB compass SHALL NOT
gain a 3D lidar in this capability.

#### Scenario: Role split

- GIVEN a coverage sweep in a region
- WHEN a coordinate is stamped onto a node
- THEN the stamper is the compass (GPS + heading), not the camera rig

### Requirement: Proximity mark is tap-range radio, not NFC on routers

Stamping a coordinate onto a node SHALL use tap-range presence already
on the compass (BLE or ESP-NOW). OpenWrt nodes SHALL NOT require NFC
silicon. A miss (no node in range) SHALL write nothing.

#### Scenario: In-range stamp

- GIVEN a compass with a GPS fix standing next to a mesh node
- WHEN the operator performs one proximity gesture
- THEN last-known coordinates for that `node_id` are written

#### Scenario: Miss

- GIVEN no mesh node in tap range
- WHEN the operator performs the same gesture
- THEN no directory coordinate is written

### Requirement: Visualizer home is web3d-space /mesh

Coverage visualization SHALL land on web3d-space `/mesh` (PlayCanvas).
hello.mesh TopologyPanel SHALL remain the RF graph. myscape SHALL NOT
be the home of this visualizer.

#### Scenario: Existing surface

- GIVEN `/mesh` already draws the simulated four-router radio world
- WHEN coverage paint lands
- THEN it is that route, not a new hello.mesh panel
