## ADDED Requirements

### Requirement: The house /128 is the operator backhaul reference

The directory, Lightning Admin's address list, and the fleet inventory SHALL present a node's house router `/128` as the backhaul reference. The string SHALL be `router_ula` in its Rust `Display` form. `10.254` SHALL stay visible on each of those surfaces as the transport pin. The directory field `backhaul_addr` and the fleet column that rollout walks SHALL stay `10.254`. A directory neighbor's `house_ula` SHALL be `router_ula` of this node's house guild id and that neighbor's node id when `/etc/mjolnir/house-guild-id` is valid, and SHALL be omitted when the file is missing or malformed. Admin SHALL show the probed `house_ula` as the reference and the probed `backhaul_addr` as the transport pin. The fleet printer SHALL use `router_ula` and SHALL NOT implement a second hash. Pending inventory lines with no node id SHALL NOT gain a `/128`. This requirement SHALL NOT change the Iroh bind, SHALL NOT assign an address, and SHALL NOT open a live radio.

#### Scenario: A neighbor shows the fixture router address

- GIVEN this node's guild file is `11` repeated to 32 bytes and a directory neighbor has node id `node-example`
- WHEN the directory is built
- THEN that neighbor's `house_ula` is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`
- AND the neighbor's transport address is still its `10.254` pin

#### Scenario: No guild file omits the reference

- GIVEN the guild file is missing
- WHEN the directory is built and Admin renders a probed node that has `backhaul_addr` `10.254.0.2` and no `house_ula`
- THEN the neighbor omits `house_ula` and Admin still shows `10.254.0.2` as the transport pin

#### Scenario: The fleet printer uses the same function

- GIVEN a member line whose node id is `node-example`, whose transport column is `10.254.0.2`, and whose guild id is `11` repeated to 32 bytes
- WHEN the fleet inventory is printed
- THEN the line shows `10.254.0.2` and `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`
- AND the walked inventory column is still the transport address

#### Scenario: An operator reads the other site

- GIVEN Admin's address list has probed a node of this house whose `house_ula` is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df` and whose `backhaul_addr` is `10.254.0.2`
- WHEN an operator reads that row
- THEN the reference they copy is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df` and `10.254.0.2` is still labeled as the transport pin
