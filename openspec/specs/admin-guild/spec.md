# admin-guild

Lightning Admin guild verbs. Product copy says guild. Wire kinds are
identikey-keyspace-v1. Radio network name stays `client-network-name`.

Code: `admin/src-tauri/src/guild.rs`, `admin/src/lib/guild/`.

## Requirements

### Requirement: Three guild verbs, protocol kinds

Lightning Admin SHALL let an operator join or change a guild, assign a
node to a guild, and nest an access-node guild inside an outer guild.
The signed identikey-log kinds SHALL be `keyspace.join`,
`keyspace.change`, `keyspace.assign-node`, and `keyspace.nest`. Admin
SHALL NOT emit a `guild.*` kind and SHALL NOT offer rename.

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

#### Scenario: No rename

- GIVEN any guild id
- WHEN the operator asks to rename
- THEN no op is appended

### Requirement: Guild signing is not a radio write

Signing a guild op SHALL NOT stage `CLIENT_SSID` and SHALL NOT invoke
the network-name apply. Applying a network name SHALL NOT append a
guild op.

#### Scenario: Membership leaves the SSID alone

- GIVEN a signed join, change, assign, or nest
- WHEN the op is appended to the local guild log
- THEN the fleet network name is unchanged
