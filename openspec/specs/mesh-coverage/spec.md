# mesh-coverage

Last-known node coordinates live on the gossiped directory projection.
A DreamBall is a signed snapshot, not the live store. The v1 surveyor
is a phone on the client SSID; visualization lives on web3d-space
`/mesh`. Folded from `add-coverage-survey` (2026-09-10). Directory
lat/lon projection folded from `add-node-coordinates` (2026-09-10).
Phone stamp UI (hello.mesh Routers panel → `POST /api/coordinate-stamp`)
folded from `add-compass-node-mark` (2026-09-10). Fixture coverage
walks (unknown / thin / covered + completeness score on `/mesh`)
folded from `add-coverage-sweep` (2026-09-10).

This capability is the architecture contract. Directory lat/lon fields
are projected. DreamBall payload and world-model overlay remain sibling
changes. First-slice `/mesh` paint of fixture walks lives in web3d-space.

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

### Requirement: Directory entries may carry a last-known stamp

`DirectoryNode` and `DirectoryNeighbor` SHALL accept an optional last-known
coordinate: WGS84 latitude and longitude, optional altitude metres, Unix
stamp time, and stamper identity. Serialization SHALL omit the coordinate
when unset (`skip_serializing_if` / equivalent). Older hello.mesh readers
SHALL remain schema-safe.

#### Scenario: Additive on a named node

- GIVEN neighbor `wr3000s-a` with a stamp `{lat, lon, stamped_at, stamper}`
- WHEN `directory.json` is written
- THEN that neighbor object includes those fields and still includes
  `node_id` and `backhaul_addr`

#### Scenario: Unmarked neighbor

- GIVEN neighbor `m3000` with no stamp
- WHEN `directory.json` is written
- THEN that neighbor object has no lat/lon keys

#### Scenario: Join to radio

- GIVEN a directory neighbor with `backhaul_addr` `10.254.242.84` and a stamp
- WHEN a consumer also has that node's `GET /api/radio`
- THEN the coordinate joins on `backhaul_addr` without reading lat/lon
  from the radio document

### Requirement: A phone stamp writes last-known coordinates

hello.mesh on a node's LAN gateway SHALL let an associated phone pick a
node from a nearby-narrowed list (associated node default; others ranked
by radio strength when known) and enter GPS (geolocation or typed). A
confirmed stamp SHALL POST a signed claim that hello verifies and spools
for meshd. An empty list, a cancel, a missing GPS with nothing typed, or
a bad signature SHALL write nothing.

#### Scenario: Nearby pick

- GIVEN a phone associated to node A's client AP with GPS available
- WHEN the operator confirms node A (the default) and the GPS
- THEN node A's directory coordinate equals that GPS and names the
  phone key as stamper

#### Scenario: Pick a neighbor

- GIVEN nearby radio strength shows node B stronger or the operator
  is standing at B
- WHEN the operator selects B from the narrowed list and confirms GPS
- THEN node B's directory coordinate is written, not A's

#### Scenario: Miss

- GIVEN the operator cancels, or the list is empty, or GPS is missing
  and the typed fields are empty
- WHEN no stamp is confirmed
- THEN directory coordinates are unchanged

### Requirement: A walk paints region coverage

Geolocated RSSI samples SHALL paint a region as unknown, thin, or covered.
A walk that heard strong client-SSID (or joined node radio) signal SHALL
mark covered. A walk that heard nothing SHALL leave unknown. Completeness
SHALL be a visible score on `/mesh`, not a hidden metric.

#### Scenario: Strong walk

- GIVEN a recorded walk whose samples are strong RSSI through a cell
- WHEN `/mesh` consumes the walk
- THEN that cell is covered

#### Scenario: Silence

- GIVEN a recorded walk whose samples are missing or below the thin
  threshold
- WHEN `/mesh` consumes the walk
- THEN those cells stay unknown

#### Scenario: Score

- GIVEN a region with a mix of covered and unknown cells
- WHEN `/mesh` is showing the sweep
- THEN completeness is on-screen
