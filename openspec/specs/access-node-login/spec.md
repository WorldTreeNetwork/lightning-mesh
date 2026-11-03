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

### Requirement: A holder admits that person on the access guild

An admin of an access-node guild SHALL be able to sign a `keyspace.join` that records one X25519 encapsulation key on that guild at the current epoch. The join SHALL add that key to the roster as a non-admin member and to the wrap list. The join SHALL NOT replace the access guild, SHALL NOT change its epoch, and SHALL NOT carry a sealed guild secret. The same call on a house guild SHALL be refused. The signer's own signing key SHALL be refused as the recipient.

#### Scenario: Admit keeps the access guild and adds the person

- GIVEN an access-node guild nested under a house, with one admin on its roster
- WHEN the holder admits a distinct 32-byte encapsulation key
- THEN the access guild still has the same id, profile, outer guild, and epoch
- AND the roster has the admin and one non-admin member for that key
- AND the wrap list names that key at the same epoch
- AND the house roster and assigned nodes are unchanged

#### Scenario: A house guild refuses the admit

- GIVEN a house guild id
- WHEN the holder admits a key on that id
- THEN no op is appended
- AND the error says to admit on the access guild
