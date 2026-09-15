# Household trust contract

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

A shipped house-owned mesh cannot depend on the developer's SSH key. Claiming,
delegating, revoking and recovering must have one explicit authority boundary
that remains valid when the entry router, browser or discovery record is hostile.
Membership and an attractive role label must not become router administration.

## What

- Define the household-authority contract used by owner claim, mesh capability
  grants, signed control and verified discovery records.
- Separate owner authority, issuer authority, operation holder and keyspace membership.
- Define denial and recovery outcomes for partitions, replay and uncertain time.

## Impact

- Capability: ADDED household-authority (proposed contract, not a shipped verifier).
- ADR: design.md records the trust-boundary decision for later architecture folding.
- Work node: mjolnir-mesh-ai0.3; activation inherited from Duke's `activate ai0`.
- Independent review is required before dependent implementation consumes this contract.
- Post-review human decision: recovery requires an enrolled credential plus physical
  access to a node. The earlier review remains historical; the amended recovery
  ceremony must be included in the next review before claiming implementation.

## User journey & surfaces

No new UI because this slice defines the contract for Lightning Admin's setup and
visible Manage network entry, rather than shipping those screens. A purchaser
claims using device proof and physical presence in the installed app; an owner
delegates a narrower grant; the destination rejects an altered target or holder.
An unclaimed node offers setup, failed verification preserves prior ownership,
and an offline node explicitly reports revocation/recovery uncertainty. Public
hello.mesh browsing and internet access never require an administrative identity.

## Out of scope

- Production claiming ceremony and factory proof provisioning: bf7.1.
- Biscuit runtime, canonical encodings and executable negative vectors: ai0.1.
- Signed operation transport, replay storage and receipts: b6j.2.
- Authenticated replicated-record implementation: ai0.9.
- Keyspace runtime, browser passkeys and paid transit remain separately gated.
- Live-network changes require separate approval; this slice does not touch nodes.
