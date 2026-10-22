# private-household-wifi

## ADDED Requirements

### Requirement: The open client SSID stays until a committed plan

The house intention SHALL record private resident Wi-Fi as the default
profile. Recording that intention SHALL NOT change the deployed open
client SSID. Only a committed resident plan SHALL replace that SSID.

#### Scenario: Decision recorded, radios untouched

- GIVEN the deployed client SSID is open
- WHEN the house intention is recorded with `commit` false
- THEN the plan's radio effect is unchanged
- AND no passphrase is required to associate

#### Scenario: Owner commits the resident plan

- GIVEN a resident plan the owner marked `commit`
- WHEN the transaction is accepted
- THEN association to the resident SSID requires that plan's passphrase
- AND the previous open SSID is no longer the committed client network

### Requirement: Resident secrets stay out of revisions and the hub

A resident or guest passphrase SHALL be named only as a root-only file
outside `wireless`, `network`, `firewall`, and `mjolnir`. Receipts and
hello.mesh responses SHALL NOT contain the passphrase. A guest profile
SHALL NOT reuse the resident secret. Creating an identity SHALL NOT be
required to use either network.

#### Scenario: Receipt is shown to the hub

- GIVEN a committed resident plan
- WHEN a receipt or hello.mesh response is produced
- THEN the passphrase and the secret file's bytes are absent

### Requirement: Guest isolation is a gate, not a label

The system SHALL offer guest Wi-Fi only as an optional second profile.
It SHALL label that profile isolated only when the same plan denies
guest reachability to resident addresses on the same node and on other
nodes, for IPv4 and for IPv6 when IPv6 is enabled, and does not bridge
`br-lan`. A plan that cannot state those denies SHALL be refused and
SHALL leave the previous revision in place. The only permit SHALL be an
explicit named service exception in that plan.

#### Scenario: Guest reaches a resident service on another node

- GIVEN an isolated guest profile and a resident service on a different node
- WHEN a guest client addresses that service over IPv4 or enabled IPv6
- THEN the plan denies it
- AND a named exception in the same plan is the only permit

#### Scenario: IPv6 is enabled and the plan has no IPv6 deny

- GIVEN the house has IPv6 enabled
- WHEN a guest plan claims isolation without an IPv6 deny
- THEN the plan is refused
- AND the previous revision remains

### Requirement: The builder does not submit to the live adapter

The resident and guest builders SHALL be pure functions. They SHALL NOT
call `OpenWrtAdapter`. They SHALL NOT bridge client L2 segments across
nodes.

#### Scenario: Builder is invoked in tests

- GIVEN a fake adapter
- WHEN the resident plan is built
- THEN `OpenWrtAdapter` is not constructed or called
