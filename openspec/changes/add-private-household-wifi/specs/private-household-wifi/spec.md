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
- THEN the committed `wireless` plan has no open client `wifi-iface`
- AND the resident `wifi-iface` is `psk2`, has no `option key`, and
  names the secret file through `wpa_psk_file`
- AND whether stations can associate is left to hardware qualification

### Requirement: Resident secrets stay out of revisions and the hub

A resident or guest passphrase SHALL be named only as a root-only file
outside `wireless`, `network`, `firewall`, and `mjolnir`. The
`wifi-iface` SHALL carry no `key` and SHALL name that file through
`wpa_psk_file`. The profile id SHALL be derived from the SSID and the
role, never from the secret. The transaction SHALL snapshot and
restore the secret file. A passphrase-only change SHALL move the
revision by a `mjolnir` generation counter and SHALL name the
`wifi-iface` whose `wpa_psk_file` hostapd re-reads on apply and on
restore. No revision, manifest, or
receipt SHALL carry an unkeyed hash of the file. Receipts and
hello.mesh responses SHALL NOT contain the passphrase. A guest profile
SHALL NOT reuse the resident secret. Creating an identity SHALL NOT be
required to use either network.

#### Scenario: Receipt is shown to the hub

- GIVEN a committed resident plan
- WHEN a receipt or hello.mesh response is produced
- THEN the passphrase and the secret file's bytes are absent

### Requirement: Guest isolation is a gate, not a label

The system SHALL offer guest Wi-Fi only as an optional second profile.
It SHALL label that profile isolated only when forwarded guest
destinations in `10.42.0.0/16` and `10.254.0.0/16` are denied, and
every house node rejects INPUT from the committed guest-source class
except DHCP and DNS on its own guest interface. Destinations outside
those aggregates that are not an address of a house node SHALL stay
allowed. The guest-source class SHALL be the CIDRs listed in each house node's
committed `mjolnir` config. Each CIDR SHALL be a CRDT claim under
`/subnets/{cidr}` supplied to the builder, never a prefix the builder
invents. The builder SHALL enable a guest `wifi-iface` on a node only
when every house node's committed config, passed in as input, already
rejects INPUT from that node's guest CIDR. A missing, unreachable, or
stale node SHALL refuse the plan. A newly joined node SHALL NOT count
as covered until its committed config carries the class. The default guest plan SHALL NOT name hello.mesh as an
exception. IPv6-enabled SHALL be read from the committed config.
Unknown SHALL refuse the plan. A guest profile on an IPv6-enabled
house SHALL be refused in this change. Enabling IPv6 while a guest
profile is committed SHALL be refused unless that revision drops the
guest profile. Guest prefixes SHALL NOT be bridged onto `br-lan` and
SHALL NOT be installed or exported by roam. A plan that cannot state
those denies SHALL be refused and SHALL leave the previous revision
in place. The only extra permit SHALL be an explicit named service
exception in that plan.

#### Scenario: Guest on a satellite still has an internet forward allow

- GIVEN an isolated guest profile on a node that is not the gateway
- WHEN the guest addresses a destination that is outside `10.42.0.0/16`
  and `10.254.0.0/16` and is not an address of a house node
- THEN the plan's forward policy allows it
- AND the return route for the guest CIDR is a meshd follow-up, not
  this plan

#### Scenario: Gateway has not committed the guest class

- GIVEN a satellite guest plan and a gateway whose committed config
  does not reject INPUT from that guest CIDR
- WHEN the builder runs
- THEN the plan is refused
- AND the previous revision stays

#### Scenario: Satellite guest addresses the gateway WAN

- GIVEN a guest on a satellite and a gateway WAN or recovery address
  configured on that gateway
- WHEN the guest addresses that address
- THEN the plan denies it as INPUT from the guest-source class
- AND a resident client address in `10.42.0.0/16` is still denied

#### Scenario: Guest reaches a resident service on another node

- GIVEN an isolated guest profile and a resident service on a different node
- WHEN a guest client addresses that service over IPv4 or enabled IPv6
- THEN the plan denies it by the house aggregates, not by a peer list
- AND a named exception in the same plan is the only extra permit

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
