## ADDED Requirements

### Requirement: A proximity stamp writes last-known coordinates

The mesh SHALL accept a stamp `{node_id, lat, lon, optional alt, stamped_at, stamper}`
from a tap-range compass gesture (BLE or ESP-NOW). A valid stamp SHALL
update that node's last-known coordinate on the directory projection. A
miss, a missing GPS fix, or an unknown `node_id` SHALL write nothing.

#### Scenario: In-range

- GIVEN a compass with a GPS fix and a mesh node in tap range whose
  `node_id` is known
- WHEN the operator performs one proximity gesture
- THEN that node's directory coordinate equals the compass fix and names
  the compass as stamper

#### Scenario: Miss

- GIVEN no mesh node in tap range
- WHEN the operator performs the same gesture
- THEN directory coordinates are unchanged

#### Scenario: No fix

- GIVEN a mesh node in range and no GPS fix on the compass
- WHEN the operator performs the gesture
- THEN directory coordinates are unchanged
