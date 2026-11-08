# wifi-internet-uplink

One authorized node joins a selected upstream Wi-Fi as a station and feeds
the mesh. Credentials are candidate-bound. A DHCP lease is not downstream
internet. Integrates with `network-change-transactions` and `mjolnir-apply`.
Folded from `add-wifi-internet-uplink` (2026-10-22) and
`add-live-uplink-probes` (2026-11-08).

Code: `crates/mjolnir-apply/src/uplink.rs` (`OpenWrtAdapter` implements
`ProvesUpstream`; tests inject probes).

## Requirements

### Requirement: Station credentials are candidate-bound
A station uplink SHALL bind its credentials to one authorized candidate:
node id, radio id, and selected upstream SSID. Using those credentials on a
different node or radio SHALL be refused. A different BSSID for the same
SSID SHALL be a different candidate.

#### Scenario: Credentials tried on the other node
- GIVEN credentials authorized for one node and radio
- WHEN a plan applies them on another node
- THEN the plan is refused before association

### Requirement: A DHCP lease is not downstream internet
A station transaction SHALL NOT commit as downstream-internet success because
the station received a DHCP lease. The receipt SHALL use network-change
transaction outcomes. Client internet SHALL remain unknown until a qualifying
downstream client-path observation is recorded.

#### Scenario: Lease without a client observation
- GIVEN a station that received an upstream DHCP lease
- WHEN no downstream client-path observation is recorded
- THEN the receipt does not claim client internet

### Requirement: Failure restores the previous plan
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

### Requirement: Passphrases stay out of receipts
The uplink passphrase SHALL live in a root-only file outside the four
revisioned UCI files. The transaction SHALL snapshot and restore that file.
Controller-visible revisions and receipts SHALL NOT include the file's bytes
or an unkeyed hash of them. hello.mesh responses SHALL NOT include the
passphrase.

#### Scenario: Receipt after a test
- GIVEN a completed uplink transaction
- WHEN the controller reads the receipt or the configuration revision
- THEN neither value confirms a guessed passphrase

### Requirement: Unselected and house networks are not upstream
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
