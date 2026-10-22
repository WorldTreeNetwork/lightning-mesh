# add-control-record-integrity

> **ACTIVE BUILD**

**Rigor:** instrument

## Why

Admin decisions that trust a gossiped record are wrong when only the ingesting
node checked it. A forged or replayed record must fail on a node that never
saw the original, and a node-vouched observation must not be readable as an
owner signature.

## What

- Add the admin-record-integrity contract for facts the control path relies on.
- Consumers verify owner-signed records themselves.
- Keep node-vouched facts (including leased-name ingest from beads p43 and 71x)
  explicitly node-vouched.
- Fail closed on forgery, replay, a withdrawn issuer, a forked authority head,
  or unreadable freshness state.
- Reuse the mesh-admin-capabilities canonical encoder. Do not define a second one.

## Impact

- Capabilities: ADDED admin-record-integrity.
- ADRs: none. design.md records the verification pins.
- Work node: mjolnir-mesh-ai0.9. Activation inherited from Duke's `activate ai0`
  (2026-09-14, repeated 2026-10-21 and 2026-10-22), bounded to this dependency
  graph. No live network writes.
- HTTPS alias resolution records stay in `add-https-aliases`.
- Writing a new authority head stays in `add-household-owner-claim`.

## User journey & surfaces

No new UI because this slice is the check Lightning Admin and meshd already
owe before they treat a discovery or control record as authority. An owner
sees a verified record replicate. A neighbor that only repeated gossip cannot
make a second node accept a forged record. Lost freshness is reported as
denied, not as a healthy admin state.

## Out of scope

- HTTPS alias label-to-address records and their issuance (`add-https-aliases`).
- Implementing Biscuit grant checks (`mjolnir-mesh-ai0.1`); this change consumes
  that encoder and does not replace it.
- The physical claim ceremony (`mjolnir-mesh-bf7.1`).
- Live fleet writes and folding `add-household-network-management`.
