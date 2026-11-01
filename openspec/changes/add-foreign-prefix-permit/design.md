# Design

## Record

`keyspace.admit-prefix` is a sibling of the existing identikey-log kinds. It is not a `guild.*` kind. The signer is an Ed25519 actor, the same way `keyspace.assign-node` is signed. The op target is the admitting house's 32-byte guild id.

The CBOR body is a map:

- `keyspace`: the admitting guild id, 32 raw bytes.
- `peer`: the other house's guild id, 32 raw bytes.
- `prefix`: that peer's house `/48`, exactly 6 bytes, `fd` plus the first five bytes of the house digest from `house_ula_prefix`.

The peer is the other house's guild id. It is not an Iroh node id. A later route may aim at a node that belongs to that house. This record does not name that node. The op's `target_fp` is the admitting guild id and MUST equal body `keyspace`. A missing target, or a target that differs from `keyspace`, is not a direction.

Field lengths are exact: `keyspace` and `peer` are 32 bytes, `prefix` is 6 bytes. A shorter or longer field is not a direction.

## Authority

The guild-log projection is not authority. `apply_op` folds joins and changes without checking the actor, and `sign_guild` will sign a caller-supplied guild id. A verified signature alone does not let that actor speak for a house.

The predicate takes a caller-supplied authority snapshot. Each entry is a guild id, an actor public key, and a role. The caller is asserting that this snapshot is already trusted. This change does not derive the snapshot from the log, and it does not transport one.

A direction counts only when the snapshot has an entry for the admitting guild whose actor is the op's actor and whose role is `admin`. Any other role, including `member`, does not count. A guild with no entry, or a snapshot the caller did not supply, does not count. The check is at predicate time and fails closed.

The fixture snapshot is: guild `11` repeated, actor `a1` repeated, role `admin`; guild `22` repeated, actor `b2` repeated, role `admin`. An unrelated actor is `ee` repeated and is not in the snapshot.

There is no expiry and no withdrawal op. A direction is absent only when the caller does not present that op. Consumers must not treat a `/48` alone as a unique house identity; the guild id is the identity. Collisions stay undetected, as in the house-prefix derivation.

## Predicate

A direction counts only when all of these hold: the signature verifies, `target_fp` is present and equals body `keyspace`, the actor is an `admin` of that guild in the trusted snapshot, `peer` is not `keyspace`, the fields have the lengths above, and `prefix` equals `house_ula_prefix(peer)`. Any other prefix, including a stock OpenWrt `ula_prefix`, does not count.

A pair is admitted only when both directions count: A names B's `/48`, and B names A's `/48`. One direction admits nothing. The predicate returns the two guild ids and the two prefixes. It does not write a route, a TUN address, or a Babel filter.

Two sites of one house share a guild id, so they fail the `peer != keyspace` check. They are not a foreign admission.

Appending the op must not change the projected roster, profile, epoch, or assigned nodes. The log reader today rejects an unknown kind and then cannot list the guild. It must accept this kind and leave that projection unchanged.

## Fixture

Guild `11` repeated to 32 bytes has house prefix `fd81:2b22:8aca::/48`. Guild `22` repeated to 32 bytes has house prefix `fd8f:78a9:2caa::/48`. A's admit of B carries the second prefix. B's admit of A carries the first.

## Unverified hypotheses

None. The spike was skipped. Cross-site forwarding and the Admin TUN route are later nodes, not premises of this record.
