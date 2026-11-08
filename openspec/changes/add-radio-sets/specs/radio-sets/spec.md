## ADDED Requirements

### Requirement: A radioset assigns one function per phy
A radioset SHALL assign each phy in the model's radio map exactly one of mesh, client-ap, station, or disabled. A set that assigns two functions to one phy SHALL be refused before any configuration change. A phy listed in the map and omitted from the set SHALL be disabled.

#### Scenario: Two functions on one phy
- GIVEN a radioset that marks one phy as both mesh and client-ap
- WHEN the set is submitted
- THEN it is refused before mutation

#### Scenario: Unlisted extra phy
- GIVEN a qualified map with three phys and a set that names two
- WHEN the set is planned
- THEN the unnamed phy is disabled

### Requirement: The default radioset is the satellite split
Until a per-model radio map is qualified, the default radioset SHALL be `satellite-split`: 5 GHz mesh backhaul and 2.4 GHz client AP. Extra mapped phys SHALL be disabled. `BACKHAUL_BAND=2g` SHALL select the 2.4 GHz mesh / 5 GHz client-ap variant of that split, fleet-wide, not a third undocumented set.

#### Scenario: Default on a dual-band board
- GIVEN an OpenWrt One with phy0 band 2g and phy1 band 5g
- WHEN the default radioset is planned
- THEN phy1 is mesh, phy0 is client-ap, and no station is assigned

### Requirement: Radiosets are not wired profiles
Radio assignment SHALL live in capability `radio-sets`, not in `node-connection-profiles`. A node-connection profile SHALL NOT list radio roles. It MAY name a radioset id. Apply SHALL project the named radioset onto UCI wireless; LuCI-edited wireless SHALL remain non-canonical.

#### Scenario: Profile without radio roles
- GIVEN ethernet-gateway-satellites with WAN on eth0 and radioset id satellite-split
- WHEN the profile is submitted
- THEN port roles come from the profile and phy functions come from the radioset

### Requirement: Station survives a radioset that assigned it
When a committed radioset marks a phy as station, wireless apply SHALL preserve that station interface and SHALL NOT treat `mode=sta` as a fatal template error. A plan that places station on a phy whose committed function is mesh SHALL be refused unless the same transaction moves mesh off that phy. The station interface SHALL NOT be bridged into the client LAN. Station credentials remain candidate-bound in `wifi-internet-uplink`.

#### Scenario: Re-apply with assigned station
- GIVEN a committed radioset with phy0 station and phy1 mesh
- WHEN apply runs with RUN_WIRELESS=1 and no radioset change
- THEN the station iface remains and the mesh phy is unchanged

#### Scenario: Station on the mesh phy
- GIVEN committed mesh on phy1
- WHEN a radioset marks phy1 station and does not move mesh
- THEN the plan is refused

### Requirement: Radioset apply is a durable transaction
A radioset change SHALL be one network-change transaction. If it does not commit, the previous radioset SHALL remain committed. A plan that changes a mesh phy SHALL require management reachability and mesh reachability before commit. The radioset id SHALL be recorded in mjolnir settings of that same plan.

#### Scenario: Failed radioset apply
- GIVEN a committed satellite-split radioset
- WHEN a replacement transaction fails before commit
- THEN satellite-split remains the committed set
