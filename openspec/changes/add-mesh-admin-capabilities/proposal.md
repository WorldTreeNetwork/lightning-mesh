# Mesh administrative capabilities

> **ACTIVE BUILD**

**Rigor:** instrument

## Why

The accepted household trust contract needs a reusable enforcement boundary before
any router control endpoint can safely consume an owner-issued grant. Association,
public identity, and token possession alone must not authorize configuration.

## What

- Add a Rust library for the mesh Biscuit application profile, with no live endpoint.
- Pin one canonical request representation shared by signer and verifier consumers.
- Enforce approved issuer, holder proof, audience, operation, epochs and expiry.
- Add negative/property tests that distinguish trusted verifier facts from token facts.

## Impact

- ADDED capability: mesh-admin-capabilities.
- Parent: ai0; work bead ai0.1; activated through Duke's ai0 activation.
- Independent instrument review required before implementation.
- ADR in design.md scopes this as a library, not a deployed administration service.

## User journey & surfaces

No new UI because the first consumer is the future signed-node-control boundary
shared by Lightning Admin and the constrained hello.mesh adapter. A valid holder
can authorize only the named operation on the named node; missing ownership state,
bad proof, stale time or failed token checks produce typed denial, never fallback
to SSH, a role label or an unsigned request. Public internet and hub access remain
outside this library's gate.

## Out of scope

- Physical owner claim/recovery credential enrollment: bf7.1.
- Live transport, durable replay reservation/execution and receipts: b6j.2.
- Replicated authority state and issuer revocation distribution: ai0.9.
- Public-browser passkeys, protected key storage, membership/keyspace runtime.
- Deployment and router configuration changes are not authorized by this change.
