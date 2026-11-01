## REMOVED Requirements

### Requirement: Three guild verbs, protocol kinds
<!-- base: 8ba6e0487fe6bfc8b717f1a8272d7b17a2a2f1ed openspec/specs/admin-guild/spec.md -->

## ADDED Requirements

### Requirement: Guild protocol kinds

Lightning Admin SHALL let an operator join or change a guild, assign a node to a guild, nest an access-node guild inside an outer guild, and admit one foreign house prefix. The signed identikey-log kinds SHALL be `keyspace.join`, `keyspace.change`, `keyspace.assign-node`, `keyspace.nest`, and `keyspace.admit-prefix`. Admin SHALL NOT emit a `guild.*` kind and SHALL NOT offer rename.

#### Scenario: Join signs keyspace.join

- GIVEN an Ed25519 actor seed and no guild id
- WHEN the operator joins
- THEN the appended op kind is `keyspace.join`
- AND the keyspace id is that op's content hash

#### Scenario: Assign and nest cite the outer guild

- GIVEN a guild id and, for assign, a 32-byte node id
- WHEN the operator assigns a node or nests
- THEN the kinds are `keyspace.assign-node` and `keyspace.nest`
- AND nest's profile is `access-node`

#### Scenario: Admit names the peer house

- GIVEN an admitting guild id, a different peer guild id, and that peer's 6-byte house prefix
- WHEN the operator admits the prefix
- THEN the appended op kind is `keyspace.admit-prefix`
- AND the body carries `keyspace`, `peer`, and `prefix`
- AND the op target equals `keyspace`

#### Scenario: No rename

- GIVEN any guild id
- WHEN the operator asks to rename
- THEN no op is appended

## MODIFIED Requirements

### Requirement: Guild signing is not a radio write
<!-- base: 8ba6e0487fe6bfc8b717f1a8272d7b17a2a2f1ed openspec/specs/admin-guild/spec.md -->

Signing a guild op SHALL NOT stage `CLIENT_SSID` and SHALL NOT invoke the network-name apply. Applying a network name SHALL NOT append a guild op. Reading a log that contains `keyspace.admit-prefix` SHALL still project join, change, assign, and nest as before. That kind SHALL NOT change the roster, profile, epoch, or assigned nodes.

#### Scenario: Membership leaves the SSID alone

- GIVEN a signed join, change, assign, nest, or admit-prefix
- WHEN the op is appended to the local guild log
- THEN the fleet network name is unchanged

#### Scenario: An admit does not reshape the guild

- GIVEN a log that already has a join and an assign
- WHEN a verified `keyspace.admit-prefix` is appended and the log is read
- THEN the roster and assigned nodes are unchanged
- AND the reader does not reject the log
