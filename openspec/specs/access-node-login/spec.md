# access-node-login

hello.mesh identikey login may ask to be wrapped into the node's
access-node guild. The client SSID does not. The guild PTK stays with
the holders (identikey-keys `wrap_login`).

Code: `crates/mjolnir-hello/src/guildspace.rs`.

## Requirements

### Requirement: Login is the ceremony

A verified `POST /api/identity` that names a 32-byte wrap recipient, on
a node whose `access-node-keyspace` file names an access-node guild,
SHALL spool a `keyspace.join` wrap-login request for that guild. The
request SHALL NOT contain a guild secret. The identikey Sign key SHALL
NOT be the recipient. A house guild id SHALL NOT be the target.

#### Scenario: SSID pass is not membership

- GIVEN a client that only hits `POST /api/portal/pass`
- WHEN the request completes
- THEN no wrap-login request is written

#### Scenario: Login names the access-node guild

- GIVEN a configured access-node keyspace id and a distinct wrap recipient
- WHEN identikey login verifies
- THEN the spool kind is `keyspace.join` and the profile is `access-node`
- AND the response names that keyspace id
