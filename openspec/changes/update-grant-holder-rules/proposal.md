# update-grant-holder-rules

> **ACTIVE BUILD**

## Why

A grant token can carry a rule whose head is `holder`. `query_all` does not run that rule until the verifier adds `time` or a holder fact, so the rule can mint the fingerprint the authority check requires. identikey-capability on protocol `main` (`e4fe668`) already refuses that rule. The mesh engine still only refuses holder facts (`9289cb0`).

## What

- Refuse a token that asserts a `holder` fact or carries a rule whose head predicate is `holder`, before `authorize` injects verifier facts.
- Keep the authority-block check, the verifier-injected fingerprint fact, and `HolderFact` as the error.
- Capability: `mesh-admin-capabilities`.

## Impact

- Capabilities: ADDED (`mesh-admin-capabilities` has no living spec yet; `add-mesh-admin-capabilities` is still open and unfolded)
- Architecture record: none. Rigor stays change.

## User journey & surfaces

No new UI because the refusal stays inside `authorize` and reaches callers only as `AuthzError::HolderFact`.

## Out of scope

- Replay reservation, receipts, hello HTTP, and iroh. Tracked on `mjolnir-mesh-ai0.10.2` / `add-signed-node-control`.
- New wire operations, unary-right changes, and the admin request CBOR.
- Folding `add-mesh-admin-capabilities` or closing `ai0`.
- Fleet deploy.

## Preparation

- Upstream: identikey-protocol `e4fe668` (holder rules) on top of `b7fef6f` (token identity). Mesh `9289cb0` rejects holder facts and requires the authority check.
- Code: `crates/mjolnir-admin-capabilities/src/engine.rs` `reject_asserted_holder` uses `query_all` only. biscuit-auth 6.0.0 `Authorizer::dump` returns rules.
- Spec: no living `mesh-admin-capabilities` spec. The open sibling change still describes fact refusal.
- Outcome: revised. The shipped check does not cover holder-headed rules.
- Authority: user “activate all” on 2026-11-04 for `mjolnir-mesh-ai0.10`. This node is dependency-ready, so the banner is ACTIVE BUILD. Bounds stay the intend non-goals: no fleet, no new wire operation, no fold of the parent capabilities change.
