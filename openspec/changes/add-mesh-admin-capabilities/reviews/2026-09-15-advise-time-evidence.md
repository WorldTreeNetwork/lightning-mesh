# Advise: ai0.1.2 time-evidence pin (trusted time / stale authority / profiles)

> **ADVISE:** send-back
> **READER:** fable-5.1-arch-review
> **SPAWN:** /home/dorje/work/WorldTree/lightning-mesh/.spawns/mjolnir-mesh-ai0.1.2-1789511000-294702-0b25e1fa

## Blind take (written from Why, living spec, codec-profile.md, `verify_holder_proof`, `internet_available` only)

1. Pin trusted time as a verifier-side input built from authenticated evidence (issuer/authority-signed time attestation, or a persisted watermark), never from the destination wall clock and never from any token or request fact; `AuthorizerBuilder::time()` may only be fed that value, or not called.
2. Pin two independent axes: token validity (expiry inside the authority block, judged against trusted time) and authority freshness (how old the destination's view of household epoch/revocations is); an unexpired token with a stale authority view is refused, and a fresh authority view with an expired token is refused, and neither axis may stand in for the other.
3. Pin per-grant-class profiles that carry a maximum token lifetime, a maximum authority staleness and a required time-evidence class; these are ceilings, so issuer-, config- or token-supplied values may only tighten them.
4. Pin that "unlimited" is a property a DECIDED class may carry, not a value operator configuration can set; a config value of none/0/unlimited outside such a class is rejected or clamped to the class cap.
5. Refuse when time evidence is missing: every absolute-validity check fails closed; only a class explicitly declared as needing no absolute time (lowest rights, e.g. diagnostics status) may proceed, and it is still bound by epoch and revocation.
6. Refuse regressed time: evidence older than the persisted high-water mark, or a monotonic anchor that shows time moved backward, denies and never lowers the watermark.
7. Refuse any time asserted by token facts, rule heads, request fields, transport headers or client clocks; the engine rejects tokens that try to derive the reserved time fact.
8. F-D under partition: the stale-authority bound is the pin's only cap on issuer-revocation exposure, so exposure for a revoked token is min(remaining token life, class staleness bound since last authority sync); a class with unlimited staleness accepts unbounded exposure and must say so.
9. Tradeoff: the overlay must stay usable offline and admin grants must not gate ordinary connectivity, so tight staleness bounds must not leave a long-partitioned household unadministrable; the escape hatch is the recovery credential or physical presence, never loosening the bound.
10. Keep pin and runtime apart: the pin names the typed inputs (trusted time value + evidence class + watermark; authority view + as-of) and the typed refusals; the engine is owed by ai0.1, replication by ai0.9, and the codec crate grows no time API at all.

## Scope and provenance

Fresh instrument review of `time-evidence.md` (ai0.1.2) limited to trusted time,
stale authority and profile parameterization. Not reviewed: codec bytes (accepted in
`2026-09-15-advise-2.md`), any runtime engine (owed by ai0.1), replication (ai0.9).
Blind take above was written before opening `time-evidence.md`, `design.md`,
`tasks.md`, the `mesh-admin-capabilities` delta, or the household-authority delta.
Verified while comparing: the profile table is identical across `time-evidence.md`,
`design.md`, the admin spec delta, `add-household-trust-contract/steer.md` and the
household-authority delta; `crates/mjolnir-admin-capabilities/src/` contains no
time, expiry or `SystemTime` symbol, so the codec slice really has no time API;
`verify_holder_proof` returns possession only. I did not re-verify the quoted
`AuthorizerBuilder::time` signature against biscuit-auth 6.0.0 source (registry
copy absent); the pin's reading matches the crate's documented behaviour as I know it.

## Where the pin answers the take (steelmanned, accepted)

- Take 1, 7: `AuthorizerBuilder::time()` forbidden, `TimeEvidence` is destination-only,
  test fixtures may freeze it but production may not accept a caller clock, and
  blocks deriving `time`/`unix_seconds`/`authority_verified_at` are rejected by
  parsed-structure inspection. Complete for the token-assertion side.
- Take 2: two axes are explicit ("A 7-day token NEVER authorizes 7 days of stale
  authority") and both directions appear as negative vectors.
- Take 3, 4: grant-class caps are listed as ceilings no profile loosens; mixed
  tokens take the shortest cap; Unlimited is named as a class property of owner
  read-only diagnostics, never a configuration grant. Owners may only shorten.
- Take 5: missing/regressed/unauthenticated time refuses privileged requests and
  leaves forwarding alone, matching the `internet_available` anchor.
- Take 9: "fresh means signed authority state, not internet" and local reissue by
  an unlocked owner signer is the offline escape hatch, not a loosened bound.
- Take 10: pin/runtime separation is honest. The pin names inputs and refusals;
  it claims no implementation and gates engine work on this review.

## Findings

Blocking findings are the send-back reason. They are additions of constraint text,
not policy questions; the DECIDED v2 profiles are not re-opened.

- **T-1 (blocking) — `TimeEvidence.unix_seconds` has no stated provenance, floor or
  ordering invariant.** `design.md` requires this library to "specify exactly what
  its trusted-time input proves"; the pin gives two `u64` fields and the phrase
  "not the node wall clock alone". "Alone" leaves the wall clock as a permitted
  component, and nothing forbids a destination from filling `unix_seconds` from
  `SystemTime::now()` after one successful authority sync. Add: (a) `unix_seconds`
  SHALL be derived from signed material (newest verified authority/revocation
  record time, or an issuer-signed time attestation bound to a destination
  challenge) plus a monotonic elapsed reading, never from wall clock; (b) a
  persisted high-water floor: evidence below the floor is "regressed" and refused,
  and the floor never decreases; (c) `authority_verified_at <= unix_seconds` else
  refuse; (d) staleness is defined as `unix_seconds - authority_verified_at`
  compared to the profile maximum, so the stale check never uses wall-clock
  subtraction. Without (b) and (d), an owner laptop with its clock set back
  produces authenticated-looking evidence that revives expired tokens and shrinks
  apparent staleness. This is the "destination treats wall clock as freshness" hole
  the acceptance criterion names.
- **T-2 (blocking) — cap and ceiling arithmetic has no issuance-time input.** The
  class-cap scenario (30-day token treated as at most 24 h) and the "Strict uplink
  past 1 h" vector require the verifier to know when the grant was issued. The pin
  lists reserved verifier-only facts but names no mandatory issuer-signed facts.
  Add: the authority block SHALL carry `issued_at` and `expires_at` (issuer claims,
  capped, never freshness); missing either refuses; `issued_at > unix_seconds`
  refuses; effective expiry is `min(expires_at, issued_at + class cap, issued_at +
  profile ceiling, parent ceiling)`; only `unix_seconds` from `TimeEvidence` feeds
  the Datalog `time` fact. Without this an engine that only evaluates the token's
  own expiry check is compliant with the pin text yet cannot enforce any cap.
- **T-3 (non-blocking, resolve in same edit) — profile source and which profile
  governs.** State that the house profile and grant class are verifier inputs read
  from verified authority state at the grant's authority epoch, not from router
  config or the token; a token-asserted profile is accepted only if not looser
  than that recorded profile. State whether the stale-authority maximum applied to
  an issued grant is the issuance-time profile or the current one ("profile changes
  apply only to new grants" reads as issuance-time; say so).
- **T-4 (non-blocking) — no-expiry diagnostics under stale authority.** The pin
  says Unlimited "ends on holder revocation or authority-epoch change" but a
  partitioned destination cannot observe either. Say explicitly that no-expiry
  applies to wall-clock expiry only, and that the profile stale-authority maximum
  and missing-time refusal still apply to these privileged reads. That is a
  clarification of the DECIDED text, not a new decision; if the author reads it
  otherwise, record the unbounded exposure in words.
- **T-5 (non-blocking) — F-D coverage.** The partition vector is present but
  single. Steer asks for it per profile. Add the Strict and Standard partition
  vectors alongside the Relaxed 7 d one, and the honest positive control: issuer
  revoked after the destination's last verify, request inside the stale window,
  token unexpired, result accept. Tests must distinguish "refused because stale"
  from "accepted inside the bounded exposure", or F-D is not actually exercised.

## What accept needs

T-1 and T-2 as constraint text in `time-evidence.md` (roughly fifteen lines), T-3
and T-4 as one sentence each, T-5 as three added vector lines. No design change,
no new human decision, no change to codec bytes or the ai0.1.1 slice. Re-review
can be targeted to the diff.
