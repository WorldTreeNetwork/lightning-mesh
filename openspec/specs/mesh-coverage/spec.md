# mesh-coverage

Last-known node coordinates live on the gossiped directory projection.
A DreamBall is a signed snapshot, not the live store. The v1 surveyor
is a phone on the client SSID; visualization lives on web3d-space
`/mesh`. Folded from `add-coverage-survey` (2026-09-10).

This capability is the architecture contract. Directory lat/lon fields,
DreamBall payload, hello stamp UI, `/mesh` paint, and the sweep game
are sibling changes.

## Requirements

### Requirement: Last-known coordinates live on the directory

Last-known node coordinates SHALL be additive optional fields on the
gossiped directory projection (`directory.json` / `GET /api/directory`).
They SHALL join to `radio.json` by `backhaul_addr` (the key both
documents already share). `radio.json` schema v1 SHALL remain RF-only.
Coordinates SHALL NOT live in UCI `mjolnir.radio` or in a central
database. Last-known SHALL be last-writer-wins on `stamped_at` plus
stamper identity.

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

#### Scenario: Stamp ingress is hello spool

- GIVEN a phone on a node's client SSID
- WHEN it submits a signed coordinate stamp
- THEN hello on that node's LAN gateway verifies the signature, spools
  the claim, and meshd ingests it into the CRDT; hello holds no key
  and is not the authority

### Requirement: DreamBall is a snapshot, not the live store

A signed DreamBall MAY carry a coverage document and a render recipe for
a viewer data-input. The live last-known coordinate SHALL remain the
directory projection. The ball SHALL be verified only through
`dreamball.wasm` (`verifyBall` then `parseBall`). Coverage SHALL be a
labeled attribute on a `ball/1` envelope, not a fourth look/feel/act axis.
Exporting a coverage ball SHALL be an explicit operator act.

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

### Requirement: Phone is the v1 surveyor; AI Camera is the world-model sensor

v1 coordinate stamps SHALL come from a phone on the client SSID: GPS
from the phone (geolocation or typed) plus a node the operator picked.
World-model capture (cameras and LiDAR) SHALL be the AI Camera path.
The LilyGo T-RGB compass SHALL NOT gain a 3D lidar in this capability.
Compass GPS SHALL NOT be a v1 dependency.

#### Scenario: Role split

- GIVEN a coverage stamp in a region
- WHEN a coordinate is written onto a node
- THEN the stamper is a phone (or later a compass with GPS), not the
  camera rig and not the OpenWrt box

### Requirement: Proximity is Wi-Fi nearby, then a pick — not NFC on routers

Stamping a coordinate onto a node SHALL use Wi-Fi: the associated
client AP plus radio strength of nearby nodes to **narrow the list**,
then the operator **selects** a node and **enters GPS**. OpenWrt nodes
SHALL NOT require NFC, BLE, or ESP-NOW silicon. BLE/ESP-NOW SHALL be
handheld-to-handheld only. An empty list, a declined pick, or missing
GPS with nothing typed SHALL write nothing.

#### Scenario: Nearby pick

- GIVEN a phone associated to a Lightning Mesh client AP, with GPS
  available or typed, and a narrowed list of nearby nodes
- WHEN the operator selects a node and confirms
- THEN last-known coordinates for that `node_id` are written via the
  hello spool

#### Scenario: Associated node is the default

- GIVEN the phone is associated to node A's client AP
- WHEN the stamp UI opens
- THEN node A is the default selection and neighbors may appear ranked
  by nearby radio strength

#### Scenario: Miss

- GIVEN no nearby node on the list, or the operator cancels
- WHEN no selection is confirmed
- THEN no directory coordinate is written

### Requirement: Visualizer home is web3d-space /mesh

Coverage visualization SHALL land on web3d-space `/mesh` (PlayCanvas).
hello.mesh TopologyPanel SHALL remain the RF graph. The stamp UI SHALL
be hello.mesh on the associated LAN gateway (select node + GPS).
myscape SHALL NOT be the home of this visualizer.

#### Scenario: Existing surface

- GIVEN `/mesh` already draws the simulated four-router radio world
- WHEN coverage paint lands
- THEN it is that route, not a new TopologyPanel

#### Scenario: Stamp UI

- GIVEN a phone on a node's client SSID
- WHEN the operator stamps a coordinate
- THEN the surface is hello.mesh on that node's LAN gateway, not `/mesh`
  and not TopologyPanel
