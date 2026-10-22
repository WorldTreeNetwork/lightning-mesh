## ADDED Requirements

### Requirement: Each port and radio has one role
A node-connection profile SHALL assign each used port exactly one of WAN,
node-link, client, or disabled, and each used radio exactly one of mesh,
client-ap, station, or disabled. A profile that assigns two roles to one
port or radio SHALL be refused before any configuration change.

#### Scenario: Shared port
- GIVEN a profile that marks one Ethernet port as both WAN and client
- WHEN the profile is submitted
- THEN it is refused before mutation

### Requirement: The default profile is the satellite split
Until a per-model port map is qualified, the default profile SHALL be
`ethernet-gateway-satellites`: Ethernet WAN on the gateway, 5 GHz mesh
backhaul, and 2.4 GHz client AP. `wired-core` SHALL be refused when that
model has no qualified map.

#### Scenario: Unqualified wired-core
- GIVEN a model with no qualified port map
- WHEN an owner applies `wired-core`
- THEN the apply is refused and names the missing map

### Requirement: Wired-core keeps the three domains apart
`wired-core` SHALL require distinct WAN, node-link, and client ports. Client
ports SHALL NOT run the mesh routing protocol. Node-link ports SHALL NOT
serve resident DHCP. WAN SHALL NOT be bridged into the client bridge. When
the profile's cable-loss mode is `isolate`, loss of the node-link SHALL NOT
move a client radio onto mesh. When the mode is `reserve-mesh`, the reserved
radio SHALL stay off client AP.

#### Scenario: Client port running babel
- GIVEN a `wired-core` plan whose client port would run babel
- WHEN the plan is checked
- THEN it is refused

#### Scenario: Cable loss isolates
- GIVEN a committed `wired-core` profile whose cable-loss mode is `isolate`
- WHEN the node-link goes down
- THEN the node does not retake a client radio for mesh

### Requirement: Profiles do not bridge client LANs or rewrite the gateway by accident
Applying a profile SHALL keep each node's client LAN as its own routed prefix.
The apply SHALL NOT add `192.168.1.1/24` to a node that does not already use
that address for recovery, and SHALL NOT remove an existing house-gateway
default-route export unless that export change is an explicit part of the
same revision-checked plan.

#### Scenario: Recovery address copied onto another node
- GIVEN a profile apply for a node that is not the recovery node
- WHEN the plan would add `192.168.1.1/24` to its client bridge
- THEN the plan is refused

### Requirement: Profile apply is a durable transaction
A profile change SHALL be one network-change transaction. If the transaction
does not commit, the previous profile SHALL remain the committed
configuration. This capability's software SHALL NOT perform a live radio write.

#### Scenario: Failed profile apply
- GIVEN a committed satellite profile
- WHEN a replacement transaction fails before commit
- THEN the satellite profile is still the committed configuration
