## MODIFIED Requirements

### Requirement: Station survives a radioset that assigned it
<!-- base: 3ff4a1911b01beb2183d1fa6e1a5fef7d65b1006 openspec/specs/radio-sets/spec.md -->
When a committed radioset marks a phy as station **and** that phy is the
owned uplink candidate, the plan builder SHALL preserve that station
interface and its secret file reference. A live `mode=sta` on a phy this
radioset does not own SHALL be refused before mutation (z3th). A role
label SHALL NOT authorize a new association or adopt an arbitrary STA.
A plan that places station on a phy whose committed function is mesh
SHALL be refused unless the same transaction moves mesh off that phy.
The station interface SHALL NOT be bridged into the client LAN. Station
credentials, default-route evidence, and `ProvesUpstream` remain
`wifi-internet-uplink` (mjolnir-mesh-21rw).

On metal, `mjolnir-apply` with `RUN_WIRELESS=1` SHALL export the committed
`mjolnir.radioset.function` table as `RADIOSET_FUNCTIONS` before invoking
`setup-wireless.sh`, so the radioset projector runs. That projector SHALL
NOT treat an owned station as a fatal template error. A live STA with no
committed radioset table SHALL be refused before the last-wins template
runs. Mesh and client-ap phys SHALL stay as the radioset assigned them.

#### Scenario: Re-apply with assigned station
- GIVEN a committed radioset with radio0 station and radio1 mesh
- AND radio0 is the owned uplink candidate
- WHEN a radioset-only re-apply is planned with no function change
- THEN the station iface and secret generation remain and radio1 stays mesh

#### Scenario: Unowned station
- GIVEN a live `mode=sta` on radio0
- AND the radioset does not mark radio0 station as the owned candidate
- WHEN the plan is checked
- THEN it is refused before mutation

#### Scenario: Station on the mesh phy
- GIVEN committed mesh on radio1
- WHEN a radioset marks radio1 station and does not move mesh
- THEN the plan is refused

#### Scenario: Metal RUN_WIRELESS preserves owned station
- GIVEN committed `mjolnir.radioset.function` includes radio0=station
- AND radio0 is `mjolnir.uplink.radio`
- AND a live sta iface is on radio0
- WHEN mjolnir-apply runs with RUN_WIRELESS=1
- THEN setup-wireless takes the radioset branch, the sta iface remains, and there is no z3th FATAL
