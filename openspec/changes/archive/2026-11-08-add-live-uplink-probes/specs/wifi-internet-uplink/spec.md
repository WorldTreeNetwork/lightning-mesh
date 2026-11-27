## MODIFIED Requirements

### Requirement: Failure restores the previous plan
<!-- base: 02f477b8e0ce5dd0cd278e18f254be7e6d4fb416 openspec/specs/wifi-internet-uplink/spec.md -->
Wrong credentials, a failed health check, or an uncertain receipt SHALL
restore the previous committed configuration through the durable transaction.
A station plan SHALL NOT install a second default route beside an existing
house-gateway export unless that export change is part of the same
revision-checked plan. Restoration health SHALL be management reachability
and the applied revision only; it SHALL NOT require upstream association,
address, route, or DNS+HTTPS.

#### Scenario: Wrong passphrase
- GIVEN a committed configuration and a station plan with the wrong passphrase
- WHEN the transaction fails
- THEN the previous committed configuration remains in effect

### Requirement: Unselected and house networks are not upstream
<!-- base: 02f477b8e0ce5dd0cd278e18f254be7e6d4fb416 openspec/specs/wifi-internet-uplink/spec.md -->
The uplink SHALL NOT join an open SSID that was not explicitly selected, and
SHALL NOT use any BSSID advertised by a mesh node as upstream. The station
section SHALL name exactly one network and SHALL NOT include a fallback list.
A radio whose committed configuration is mesh mode SHALL be refused. The
station interface SHALL NOT be bridged into the client LAN. A mesh
default-route export SHALL be installed only after upstream-bound DNS and
HTTPS evidence passes.

#### Scenario: House SSID selected as upstream
- GIVEN a BSSID advertised by a mesh node
- WHEN a plan names it as the upstream network
- THEN the plan is refused

#### Scenario: Lease on a captive upstream
- GIVEN a station that received a DHCP lease and failed upstream-bound DNS and HTTPS
- WHEN the plan would export a mesh default route
- THEN no mesh default route is installed and the previous revision is restored

#### Scenario: Mesh radio taken for station
- GIVEN a radio whose committed mode is mesh
- WHEN a station plan selects it and no profile has changed that mode in the same transaction
- THEN the plan is refused

## ADDED Requirements

### Requirement: OpenWrtAdapter proves upstream on metal
`OpenWrtAdapter` SHALL implement `ProvesUpstream`. After writing allowlisted
UCI it SHALL reload wireless. Before commit it SHALL observe station
association, address, default route on that iface, and upstream-bound DNS
plus HTTPS, and SHALL observe mesh and management reachability. A DHCP lease
alone SHALL NOT pass DNS+HTTPS. The uplink secret file SHALL be snapshotted
and restored with the transaction. `submit_uplink_plan` SHALL accept
`OpenWrtAdapter`. Field RF walks remain `mjolnir-mesh-lpv`.

#### Scenario: Passing probes commit
- GIVEN a candidate-bound station plan and probes that pass REQUIRED_HEALTH
- WHEN submit_uplink_plan runs on OpenWrtAdapter
- THEN the transaction commits and the receipt does not contain the passphrase

#### Scenario: Failed DNS restores
- GIVEN a station that associated and received a lease
- AND upstream-bound DNS+HTTPS fails
- WHEN the transaction health-checks
- THEN the previous revision is restored
