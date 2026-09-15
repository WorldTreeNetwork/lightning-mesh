# ADR proposal: explicit verifier context, no ambient authorization

> **Timing policy: security profiles (Duke, 2026-09-15; see
> [steering](../add-household-trust-contract/steer.md) and the
> [trust design timing section](../add-household-trust-contract/design.md)).**
> This supersedes every earlier same-day 900 s / 3600 s reading.
>
> | Profile | Token default | Token ceiling | Stale-authority maximum |
> |---|---:|---:|---:|
> | Strict | 15 min | 1 h | 15 min |
> | **Standard (default)** | **24 h** | **7 d** | **24 h** |
> | Relaxed (opt-in, warning) | 7 d | 30 d | 7 d |
>
> - **Grant-class caps** bound grants too, and the shortest applicable cap wins:
>   - network, radio, firewall, DNS and uplink changes: ≤ 24 h (≤ 1 h under
>     Strict)
>   - firmware install: ≤ 15 min, single transaction
>   - ownership and issuer changes: owner-only, single-use, ≤ 5 min
> - No configuration-changing or delegated Admin grant is ever unlimited.
>   No-expiry applies only to epoch-bound, read-only owner diagnostics.
> - `mjolnir-mesh-ai0.1.2` owns the authenticated time-evidence mechanism and a
>   targeted timing/profile re-review.
> - The codec/holder-proof slice has no timing or authorization API and is
>   unaffected.

The local implementation pins and first codec/proof slice are in
[codec-profile.md](codec-profile.md). Its new byte-level choices need independent
review; the earlier abstract-contract acceptance does not accept those choices.

The library implements a mesh application profile of the adopted IdentiKey Biscuit
format, not a replacement token format. Source dossier:
`openspec/changes/add-household-network-management/identity-and-welcome.md` and the
reviewed sibling protocol document `docs/standards/identikey-capability-v1.md`.
That document is a draft with missing upstream vectors. Implementation must pin
the exact source revision and distinguish local mesh vectors from upstream
conformance. Do not claim an absent IdentiKey capability runtime is available.

## Input and trust boundary

An explicitly supplied verified-authority context contains house/node identity,
approved issuer public keys and current epochs. No lookup in a token, public
directory, hostname or client-provided role may construct that context. Holder
fingerprints follow the IdentiKey self-describing public-key fingerprint contract;
raw-key hashing and arbitrary algorithm aliases are not compatible substitutes.

Separate signed-request proof verification from policy evaluation in the API, but
make it impossible for an unverified public request struct to masquerade as a
verified holder. No exported bool such as is_admin or signature_verified is an
authorization credential. Minimize trusted constructors and document integration
preconditions for downstream transport/authority consumers.

## Canonical request

One library-owned encoder covers domain/version, house, node, authority epoch,
operation, complete arguments, token identity and target-issued one-use challenge.
The byte format must be selected, pinned and fixture-tested before implementation
is accepted; reuse existing compatible IdentiKey encoding rather than define an
ad hoc serializer. Reject unsupported versions, duplicate/ambiguous fields, unknown
operations, oversized payloads and noncanonical forms as appropriate to that codec.
ai0.9 and b6j.2 consume this encoder; they must not reproduce it independently.

## Biscuit profile

Use the upstream Biscuit verification library to verify the complete chain and all
checks. Approved issuer pins are external to the token. Rights originate only in
the root authority scope; attenuation may narrow, not add authority. Every grant
requires a holder check; token-supplied holder/request/time/authority facts cannot
satisfy verifier-only predicates. Datalog trust scopes and evaluation/resource
limits must be explicit, with forged-fact vectors in both root and appended blocks.

V1 issuance is owner-approved, holder-bound and operation/node scoped. A changed
holder requires fresh authorized issuance, not attenuation. Autonomous delegated
minting is excluded initially; later reissuance must preserve parent ceilings and
revocation lineage. Do not mint grants for unsupported operations.

## Time and replay interface

Configuration grants take their token default and ceiling from the household
security profile. Standard (the default) is 24 h / 7 d. Strict is 15 min / 1 h.
Relaxed is 7 d / 30 d.

The verifier takes the profile and grant-class caps as trusted verifier input,
never from the token, and applies the shortest applicable cap. It separately
enforces the profile's stale-authority maximum (15 min / 24 h / 7 d) against the
time of its last verified authority and revocation state. It refuses mutating or
delegated Admin grants without an expiry, and allows no-expiry only for
epoch-bound read-only diagnostics.

Enforce issuer/delegation ceilings and reject arithmetic overflow. Activity does
not extend expiry. Renewal requires fresh verified authority/revocation state.
Verified time evidence must establish validity; missing or regressed evidence
refuses privileged requests. A caller-supplied wall clock is not trusted evidence.
This library must specify exactly what its trusted-time input proves and cannot
claim the platform's time-evidence primitive is implemented by a test fixture.

Library success is a bounded authorization decision, NOT permission to execute a
replayable request. b6j.2 must durably reserve the bound challenge/request-id against
the exact authorization epoch before execution and serialize against revocation.
Its missing/corrupt ledger must fail closed. Any replay abstraction in this library
must expose that obligation without a permissive production in-memory fallback.

## Red-first verification

Before implementation, add focused failing tests for wrong issuer, changed holder,
different target/operation/argument digest, appended facts that claim authority,
expired/overflowing TTL and uncertain time. Successful attenuation and original
valid-holder requests are the positive controls. Report any test that depends on
mock authority/time/replay storage as such; no fixture is proof of live recovery,
freshness distribution or durable replay behavior.

The implementation packet must pin reviewed upstream APIs/version and canonical
encoding before coding. If interoperability inputs are genuinely missing, record
that exact dependency rather than inventing compatibility or seeking another
policy approval for defaults already delegated by Duke.
