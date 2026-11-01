# foreign-prefix-permit

A foreign house prefix is admitted only by a pair of
`keyspace.admit-prefix` ops. Each direction names the admitting guild
id, the peer guild id, and that peer's house `/48`. The actor must be
an `admin` in a caller-supplied authority snapshot. One direction
admits nothing. The predicate does not write a route, a TUN address, a
Babel filter, or a client-subnet claim. Folded from
`add-foreign-prefix-permit` (2026-11-01).

Code: `admin/src-tauri/src/permit.rs`, `admin/src-tauri/src/guild.rs`.

## Requirements

### Requirement: A foreign house prefix needs both signatures

A foreign house prefix SHALL be admitted only by a pair of `keyspace.admit-prefix` ops that each count. Each op SHALL name the admitting guild id, the peer guild id, and a prefix. `keyspace` and `peer` SHALL be 32 bytes. `prefix` SHALL be 6 bytes. The op target SHALL be present and SHALL equal body `keyspace`. A direction SHALL count only when the signature verifies, the target matches, the actor is an `admin` of the admitting guild in the caller-supplied authority snapshot, the peer guild id is not the admitting guild id, and the prefix equals the peer's house `/48` from `house_ula_prefix`. The snapshot is trusted input. It SHALL NOT be taken from the guild-log projection. A missing snapshot, a guild with no entry, and any role other than `admin` SHALL NOT count. The pair SHALL count only when each guild names the other's house `/48`. One direction SHALL admit nothing. A prefix that is not that house `/48`, including a stock OpenWrt `ula_prefix`, SHALL NOT count. The predicate SHALL NOT write a route, a TUN address, a Babel filter, or a client-subnet claim. It SHALL NOT expire or withdraw an op. Two sites of one house share a guild id and SHALL NOT be a foreign admission. A `/48` alone SHALL NOT be treated as a unique house identity.

#### Scenario: Both fixture directions admit the pair

- GIVEN guild `11` repeated to 32 bytes, prefix `fd81:2b22:8aca::/48`, admin actor `a1` repeated, and guild `22` repeated to 32 bytes, prefix `fd8f:78a9:2caa::/48`, admin actor `b2` repeated, both present in the trusted snapshot
- WHEN each admin has a verified admit of the other's prefix and each op target equals that op's `keyspace`
- THEN the pair is admitted
- AND no route is installed

#### Scenario: One signature admits nothing

- GIVEN only guild `11`'s admin has a verified admit of `fd8f:78a9:2caa::/48`
- WHEN the predicate runs with that admin in the snapshot
- THEN the pair is not admitted

#### Scenario: An unrelated actor signs both directions

- GIVEN actor `ee` repeated is not in the snapshot
- WHEN that actor signs both fixture directions with matching targets and derived prefixes
- THEN the pair is not admitted

#### Scenario: A non-admin member does not speak for the house

- GIVEN the snapshot lists actor `a1` repeated for guild `11` with role `member`
- WHEN that actor signs guild `11`'s half of the fixture pair
- THEN that direction does not count

#### Scenario: Missing authority fails closed

- GIVEN both fixture admins signed matching admits
- WHEN the caller supplies no snapshot
- THEN the pair is not admitted

#### Scenario: A mismatched target does not count

- GIVEN guild `11`'s admin is in the snapshot and signs a body whose `keyspace` is guild `11`
- WHEN the op target is missing or is not guild `11`
- THEN that direction does not count

#### Scenario: A stock prefix does not match

- GIVEN both admins are in the snapshot and sign a prefix other than the peer's house `/48`
- WHEN the predicate runs
- THEN neither direction counts

#### Scenario: The same guild is not foreign

- GIVEN an authorized admin signs an op whose peer guild id equals its admitting guild id
- WHEN the predicate runs
- THEN that direction does not count
