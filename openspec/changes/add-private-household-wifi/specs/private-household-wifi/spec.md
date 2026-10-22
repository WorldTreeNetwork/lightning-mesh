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
`wpa_psk_file`. The profile id SHALL be the lowercase hex of a
domain-separated blake3 over the role and the SSID bytes, never the
secret and never the raw SSID, so the file name is a single path
component under `/etc/mjolnir/secrets/wifi/`. The secret file SHALL
be exactly one line. The transaction SHALL snapshot and
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
destinations in the committed mesh client space (default
`10.42.0.0/16`) and in `10.254.0.0/16` are denied, and
every house node rejects INPUT from the committed guest-source class
except DHCP and DNS on its own guest interface. A guest CIDR outside
that client space SHALL be refused. Destinations outside
those aggregates that are not an address of a house node SHALL stay
allowed. The guest-source class SHALL be the CIDRs listed in each house node's
committed `mjolnir` config. Each CIDR SHALL be a CRDT claim under
`/subnets/{cidr}` supplied to the builder, never a prefix the builder
invents. The builder SHALL enable a guest `wifi-iface` on a node only
when every house node's committed config, passed in as input, already
rejects INPUT from that node's guest CIDR. A missing, unreachable, or
stale node SHALL refuse the plan. A newly joined node SHALL NOT count
as covered until its committed config carries the class. A new guest
plan SHALL be refused while any house node lacks the class. The export
hold, that a joiner does not export routes while a committed
guest-source class it lacks exists, is `mjolnir-mesh-fyby.4`. This
change does not implement admission. Any later revision on a house node SHALL be refused when it
drops a guest CIDR from that node's committed class while any house
node still has a guest SSID committed for that CIDR, and the previous
revision SHALL stay. That refusal covers every builder that rewrites
`mjolnir`, not only a restore. In-crate writers that edit named
`mjolnir` sections keep the class. Hand `uci`, `meshctl`, and deploy
scripts are not builders and are outside this SHALL. The guest-enable submitter SHALL
re-read every peer revision it built against immediately before
submitting the guest SSID, and SHALL refuse if any moved. Two
transactions that both pass that check before either commits remain
a residual window. The guest interface's device SHALL set `ipv6` to
`0`, so the guest link has no IPv6 address, including link-local.
The default guest plan SHALL NOT name hello.mesh as an
exception. IPv6 SHALL count as enabled when the committed config shows
any of `network.globals` `ula_prefix`, `ip6assign` on any interface, a
`wan6` interface or `proto` `dhcpv6`, or odhcpd `ra` or `dhcpv6` server
on a client interface. Missing configs SHALL refuse the plan. A guest
profile SHALL be refused when that predicate is true. A guest
`wifi-iface` SHALL NOT set `ip6assign`. Enabling IPv6 while a guest
profile is committed SHALL be refused unless that revision drops the
guest profile. Guest prefixes SHALL NOT be bridged onto `br-lan`.
`guest_routes` SHALL NOT install or export a `/32` whose address is in
the committed guest-source class. A plan that cannot state
those denies SHALL be refused and SHALL leave the previous revision
in place. The only extra permit SHALL be an explicit named service
exception in that plan.

#### Scenario: Guest on a satellite still has an internet forward allow

- GIVEN an isolated guest profile on a node that is not the gateway
- WHEN the guest addresses a destination that is outside the committed
  mesh client space and outside `10.254.0.0/16` and is not an address
  of a house node
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
- WHEN a guest client addresses that service over IPv4
- THEN the plan denies it by the house aggregates, not by a peer list
- AND a named exception in the same plan is the only extra permit

#### Scenario: IPv6 is enabled or unknown

- GIVEN the committed config has `ula_prefix` or `ip6assign`, or those configs were not passed in
- WHEN a guest profile is planned
- THEN the plan is refused
- AND the previous revision remains

#### Scenario: A later revision drops the guest class

- GIVEN a guest SSID is committed for a CIDR
- AND a house node's committed config rejects INPUT from that CIDR
- WHEN a revision of that node removes the CIDR from its class
- THEN the revision is refused
- AND the previous revision stays

#### Scenario: Guest device has no IPv6

- GIVEN a guest profile the IPv6 predicate allows
- WHEN the plan is built
- THEN the guest interface device sets `ipv6` to `0`

#### Scenario: Guest CIDR is outside the client space

- GIVEN a claimed guest CIDR outside the committed mesh client space
- WHEN the builder runs
- THEN the plan is refused
- AND the previous revision stays

#### Scenario: A peer revision moves before submit

- GIVEN a guest plan built against a set of peer revisions
- WHEN one of those revisions has moved before submit
- THEN the submit is refused
- AND the previous revision stays

#### Scenario: A joiner has not carried the class

- GIVEN a guest SSID is already committed
- WHEN a new node's admitted config does not carry the guest-source class
- THEN that node is not covered
- AND a new guest plan is refused
- AND the export hold is `mjolnir-mesh-fyby.4`, not this plan

### Requirement: The builder does not submit to the live adapter

The resident and guest builders SHALL be pure functions. They SHALL NOT
call `OpenWrtAdapter`. They SHALL NOT bridge client L2 segments across
nodes.

#### Scenario: Builder is invoked in tests

- GIVEN a fake adapter
- WHEN the resident plan is built
- THEN `OpenWrtAdapter` is not constructed or called
