# Household owner claim

## Ceremony pin

Selected for review before implementation. No TPM, no factory attestation,
and no custom unaudited key exchange.

A claim or recovery request is accepted only when every required factor
verifies against the same request, node id, and ownership epoch.

1. **Device proof.** The node's meshd identity key signs the request:
   owner public key, node id, epoch, ceremony nonce, and window expiry.
   The node id is the key this mesh already derives addresses from.

2. **One-time provisioning secret.** Compared with a hash stored on the node
   at provision. Shown once to the purchaser, on a printed card or on that
   node's own first-claim screen. It is not a WPS PIN and not a developer key.

3. **Physical window.** The local setup button (the same button position as
   WPS on these devices) only arms presence. It opens a configurable window
   that defaults to 120 seconds, closes on success, reboot, or loss of
   trustworthy timing, and is consumed by one success. The WPS protocol is
   not used.

Secret without the button fails. Button without the secret fails. A remote
presentation of the secret fails because the window was never armed on the node.

The first complete claim is persisted atomically against the node id and moves
the epoch from unclaimed to 1. A racing second claim cannot overwrite it.

## Recovery pin

Recovery requires an enrolled recovery credential and a fresh physical window
bound to that request, node, and current epoch. Neither factor is enough.
Success, and any completed ownership transfer, retires every prior recovery
credential. The new owner re-enrolls. Enrollment is epoch-scoped.

If two partitions each complete recovery, this change may persist two candidate
heads. It does not pick a winner. `add-control-record-integrity` refuses a fork
until one successor chains from a single parent under a new physical ceremony.

## Clocks

The 120 second figure is only the physical window. Configuration grants use
the household-authority profiles (Standard default: 24 hour token / 7 day
ceiling / 24 hour stale-authority maximum) and the class caps in that contract.
This change does not reintroduce a 900 second admin-grant default.

## Keys

hello.mesh HTTP handlers never receive the owner private key, the provisioning
secret after the comparison, or a reusable admin bearer. Comparison uses the
stored hash.

## Not owed here

- Live button presses on fleet hardware (`mjolnir-mesh-lpv` qualifies hardware).
- Implementing the Biscuit engine.
- Verifying gossiped records on a non-writer. That is admin-record-integrity.
