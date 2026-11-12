# steer add-signed-node-control

**When.** 2026-11-04
**Depth.** explicit

## Decided
- Receipt: destination node key over a canonical CBOR array, domain `lightning-admin/receipt`, version 1, covering the replay obligation, the operation, and the outcome (user). Same codec family as the admin request. Not a gossip SignedRecord.
  Why: later nodes cannot swap the signed bytes without a new change.
- Ledger: a caller-supplied durable trait, with one file of reserved obligations on the destination (user). A missing, corrupt, or unreadable file refuses. Tests use a temp file. No production in-memory allow.
  Why: a restart must not forget a reservation.
- Transport: `POST /api/control` and iroh ALPN `mjolnir/admin/1`, same envelope bytes on both (user). Not `/api/identity` and not `mjolnir/mesh/v0`.
  Why: hello and iroh have to name one route or they drift.
- Tests: the hello handler and a local iroh endpoint pair (user). The entry has no node key. No fleet node and no radio.
  Why: the acceptance has to exercise both transports without a live house.

## Skipped
- (none)

## Feeds change
The destination node key signs `lightning-admin/receipt` version 1 over the replay obligation, the operation, and the outcome. Reservations live in a caller-supplied file ledger that fails closed. Hello forwards `POST /api/control` and iroh accepts `mjolnir/admin/1`, both with the same envelope. Tests use the local HTTP handler and a local iroh pair. No fleet rollout. Name publish stays out. Argument bytes stay the grant library's canonical request.

[AUTO] No new admin wire operation codes.
[AUTO] Reader for the instrument contract is Astra.
