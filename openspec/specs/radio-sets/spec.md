# radio-sets

A named assignment of every phy on a model to mesh, client-ap, station, or
disabled. Apply projects that set onto UCI wireless. Wired profiles keep
ports and may name a radioset id. Folded from `add-radio-sets` (2026-11-08).

Code: `crates/mjolnir-apply/src/node_profile.rs`,
`deploy/openwrt/setup-wireless.sh`.

## Requirements

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

### Requirement: Phys resolve by map id and path
A radio-map phy `id` SHALL be the UCI wifi-device name. Hardware identity
SHALL be the mac80211 `path`. `band` SHALL constrain that phy and SHALL
NOT pick among same-band devices. Apply SHALL match inventory to the map
by `path`, then require the UCI name to equal `id`. Wrong model, missing
required path, duplicate path, assignment to an unknown id, or two map
entries that match one device SHALL be refused before mutation.
Unqualified maps MAY still supply identity for the default radioset.
A detected phy whose path is absent from the map SHALL be unmanaged
(dongle owner) unless it already carries a managed wifi-iface this
radioset would own, in which case the plan is refused. A mapped phy
omitted from the set SHALL be disabled, which is distinct from unmanaged.

#### Scenario: Reordered UCI names
- GIVEN a map id radio1 path platform/soc/18000000.wifi+1
- AND inventory presents that path under a different wifi-device order
- WHEN the default radioset is planned
- THEN the 5 GHz path is still mesh and the 2.4 GHz path is still client-ap

#### Scenario: Two same-band phys
- GIVEN a map with two 5 GHz phys that have distinct paths and ids
- WHEN satellite-split is planned
- THEN only the id the definition names as mesh is mesh; the other is not inferred

#### Scenario: Map inventory mismatch
- GIVEN a map path that is not present on the node
- WHEN a radioset is planned
- THEN it is refused before mutation

### Requirement: The default radioset is the satellite split
Until a per-model radio map is qualified, the default radioset SHALL be
`satellite-split`: the mapped 5 GHz phy is mesh and the mapped 2.4 GHz phy
is client-ap. Extra mapped phys SHALL be disabled. The 2.4 GHz mesh variant
SHALL be a distinct definition id `satellite-split-2g`, not an environment
flag at replay. `BACKHAUL_BAND=2g` SHALL select `satellite-split-2g` only
on first adoption when no radioset id is committed. An absent radioset id
SHALL mean `satellite-split` and SHALL be a no-op on today's satellite
fixture. An unknown id SHALL be refused. A profile that names a radioset
id SHALL take precedence over `BACKHAUL_BAND`.

#### Scenario: Default on a dual-band board
- GIVEN an OpenWrt One whose radio map lists radio0 band 2g path
  platform/soc/18000000.wifi and radio1 band 5g path
  platform/soc/18000000.wifi+1
- WHEN the default radioset is planned
- THEN radio1 is mesh, radio0 is client-ap, and no station is assigned

#### Scenario: 2g variant survives without the selecting environment
- GIVEN a committed radioset id `satellite-split-2g` with resolved
  functions radio0=mesh and radio1=client-ap
- WHEN apply re-runs with no BACKHAUL_BAND in the environment
- THEN the committed 2g assignment is planned again

#### Scenario: Failed 2g selection restores that selection
- GIVEN a committed `satellite-split-2g` resolved assignment
- WHEN a replacement radioset transaction fails before commit
- THEN `satellite-split-2g` and its resolved functions remain committed

#### Scenario: Unknown radioset id
- GIVEN a profile that names radioset id `not-a-set`
- WHEN the plan is checked
- THEN it is refused before mutation

### Requirement: Radiosets are not wired profiles
Radio assignment SHALL live in capability `radio-sets`, not in `node-connection-profiles`. A node-connection profile SHALL NOT list radio roles. It MAY name a radioset id. Apply SHALL project the named radioset onto UCI wireless; LuCI-edited wireless SHALL remain non-canonical.

#### Scenario: Profile without radio roles
- GIVEN ethernet-gateway-satellites with WAN on eth0 and radioset id satellite-split
- WHEN the profile is submitted
- THEN port roles come from the profile and phy functions come from the radioset

### Requirement: Station survives a radioset that assigned it
When a committed radioset marks a phy as station **and** that phy is the
owned uplink candidate, the plan builder SHALL preserve that station
interface and its secret file reference. A live `mode=sta` on a phy this
radioset does not own SHALL be refused before mutation (z3th). A role
label SHALL NOT authorize a new association or adopt an arbitrary STA.
A plan that places station on a phy whose committed function is mesh
SHALL be refused unless the same transaction moves mesh off that phy.
The station interface SHALL NOT be bridged into the client LAN. Station
credentials, default-route evidence, and `ProvesUpstream` remain
`wifi-internet-uplink` (mjolnir-mesh-21rw). Wiring the shell
`setup-wireless.sh` so an owned station is not a FATAL on metal is
mjolnir-mesh-9d3o; this change owns the refusals and the plan-builder
preserve.

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

### Requirement: Radioset apply is a durable transaction
A radioset change SHALL be one network-change transaction. The committed
record SHALL include the definition id, the variant id when it is not the
5 GHz default, and the resolved per-phy function table. If the transaction
does not commit, that exact record SHALL remain committed. A plan that
changes a mesh phy SHALL require management reachability and mesh
reachability before commit. Until OpenWrtAdapter implements those probes,
submit of a radioset **change** to the live adapter SHALL be refused; a
fake-adapter plan MAY still be built. The shell projector SHALL NOT commit
UCI or restart wifi outside that transaction. Client SSID and encryption
SHALL remain `mjolnir.radio`. Stale enabled wifi-ifaces on a phy the set
disables SHALL be disabled in the plan. The existing disabled `clientap2g`
section SHALL stay rendered-disabled when that phy is not client-ap.

#### Scenario: Failed radioset apply
- GIVEN a committed satellite-split radioset with its resolved function table
- WHEN a replacement transaction fails before commit
- THEN that id and table remain the committed set

#### Scenario: Composed profile and radioset rollback
- GIVEN one transaction that names a profile id and a radioset id
- WHEN the transaction fails before commit
- THEN both the previous profile and the previous radioset record remain

#### Scenario: Live adapter without probes
- GIVEN a radioset plan that would change a mesh phy
- AND OpenWrtAdapter does not implement the required probes
- WHEN submit is attempted on that adapter
- THEN it is refused and no UCI is written
