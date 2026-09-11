## ADDED Requirements

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
