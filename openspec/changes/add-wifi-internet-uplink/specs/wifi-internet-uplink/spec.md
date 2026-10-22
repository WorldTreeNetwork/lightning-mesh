## ADDED Requirements

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
revision-checked plan.

#### Scenario: Wrong passphrase
- GIVEN a committed configuration and a station plan with the wrong passphrase
- WHEN the transaction fails
- THEN the previous committed configuration remains in effect

### Requirement: Passphrases stay out of receipts
The uplink passphrase SHALL be stored by reference. Receipts, controller-facing
logs, and hello.mesh responses SHALL NOT include it.

#### Scenario: Receipt after a test
- GIVEN a completed uplink transaction
- WHEN the controller reads the receipt
- THEN the receipt has no passphrase

### Requirement: Unselected and house networks are not upstream
The uplink SHALL NOT join an open SSID that was not explicitly selected, and
SHALL NOT use the house's own client SSID as upstream. The station interface
SHALL NOT be bridged into the client LAN.

#### Scenario: House SSID selected as upstream
- GIVEN the configured house client SSID
- WHEN a plan names it as the upstream network
- THEN the plan is refused
