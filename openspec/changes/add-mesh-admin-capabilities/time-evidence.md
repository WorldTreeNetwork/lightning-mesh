# Authenticated time evidence — implementation pin (ai0.1.2)

Status: proposed local pin under this ACTIVE change. Independent review is
required before the authorization engine consumes it. Codec/holder-proof
(`codec-profile.md`, `mjolnir-mesh-ai0.1.1`) has no time API and is unchanged.

Policy numbers are owner-profile parameters inside the accepted household
authority contract, not a new trust-boundary. Numeric 900 s / 3600 s from
earlier same-day steering is superseded. Do not implement those values.

## Profiles (DECIDED v2, 2026-09-15)

Configuration-changing admin grants carry three durations, selected by house
profile. Profile changes apply only to newly issued grants.

| Profile | Token default | Token ceiling | Max stale authority |
|---|---|---|---|
| Strict | 15 min | 1 h | 15 min |
| STANDARD (default) | 24 h | 7 d | 24 h |
| Relaxed (opt-in, persistent risk warning) | 7 d | 30 d | 7 d |

Owners may shorten either timer. Lengthening requires a profile switch, not a
policy patch on an issued grant. Unlocked owner signer may reissue locally
without internet. Delegates need owner-authorized reissue.

**Unlimited** is not a configuration-grant profile. It is only private
read-only diagnostics on the owner's registered holder-bound device. It
waives wall-clock token expiry only. The profile stale-authority maximum
and missing-time refusal still apply to these privileged reads. Holder
revocation or authority-epoch change ends them once the destination has
verified that fact; a partitioned destination cannot observe either, so
exposure is bounded by stale-authority, not by "no expiry". Never for
configuration-changing or delegated Admin grants.

House profile and grant class are verifier inputs from verified authority
state at the **grant's issuance epoch**. A token-asserted profile is
accepted only if it is not looser than that recorded profile. Profile
changes apply only to newly issued grants: the stale-authority maximum
for an already-issued grant is the issuance-time profile.

**Grant-class caps** (no profile loosens these):

- Primary Wi-Fi / backhaul / radio / routing / firewall / DNS / uplink: ≤24 h
  token (≤1 h Strict); stale ≤24 h.
- Firmware / software install: single transaction ≤15 min, bound to artifact
  digest; owner approval by default.
- Ownership / issuer / recovery-policy / protected-key changes: owner-only,
  single-use, ≤5 min.
- Guest Wi-Fi and constrained device management follow the house profile.
- Mixed tokens take the shortest applicable cap.

`fresh` means locally verifiable signed authority/revocation state, not
reachability of the internet. Ordinary connectivity and hello.mesh identity
are unaffected.

## What time evidence must prove

A destination MAY authorize a privileged operation only when all of:

1. Effective expiry is in the future under **authenticated** `unix_seconds`
   (see below). The node wall clock is not an input.
2. Staleness `unix_seconds - authority_verified_at` is within the
   issuance-time profile's **max stale authority** window.
3. Grant-class cap, parent ceiling, and profile ceiling are all respected
   using issuer-signed `issued_at` / `expires_at`.
4. Renewal is a new authorization check, not activity-extended lifetime.

A 7-day token NEVER authorizes 7 days of stale authority. Missing, regressed,
or unauthenticated time refuses the privileged request and does not disturb
ordinary forwarding.

## Trusted-time input (engine, not codec)

Do not call `AuthorizerBuilder::time()` (`biscuit-auth` 6.0.0). That method
adds a fact from the process clock. Documented:

```rust
impl AuthorizerBuilder {
    pub fn time(self) -> Self; // "adds a fact with the current time"
    pub fn set_limits(self, limits: AuthorizerLimits) -> Self;
}
```

The engine MUST inject verifier-only facts from an explicit
`TimeEvidence` value produced by a destination-owned bootstrap:

```rust
pub struct TimeEvidence {
    unix_seconds: u64,          // 0..=i64::MAX
    authority_verified_at: u64, // last successful authority/revocation verify
}
```

`unix_seconds` SHALL be derived from signed material (newest verified
authority/revocation record time, or an issuer-signed time attestation bound
to a destination challenge) plus a monotonic elapsed reading. It SHALL NOT
be filled from `SystemTime::now()` or any other wall clock, including after
a successful authority sync. `authority_verified_at <= unix_seconds` else
refuse. Staleness is only `unix_seconds - authority_verified_at` compared
to the issuance-time profile maximum; never wall-clock subtraction.

A persisted high-water floor stores the greatest `unix_seconds` accepted.
Evidence below the floor is regressed and refused. The floor never decreases.
Clock rollback or loss of monotonic window: previous evidence is not restored;
privileged use fails closed until new authenticated evidence exists.

Construction is destination-only. Test fixtures may supply frozen evidence;
production must not accept a caller wall clock as that type. Token blocks
MUST NOT be allowed to derive `time()`, `unix_seconds`, or
`authority_verified_at`. Inspect parsed blocks for reserved fact/rule heads
(same rule as codec-profile: not substring matching). Only `unix_seconds`
from `TimeEvidence` may feed the Datalog `time` fact.

The authority block SHALL carry issuer-signed `issued_at` and `expires_at`
(claims, never freshness). Missing either refuses. `issued_at > unix_seconds`
refuses. Effective expiry is
`min(expires_at, issued_at + class cap, issued_at + profile ceiling, parent ceiling)`.
Without those facts an engine that only evaluates the token's own expiry
check cannot enforce any cap.

## Negative vectors owed before engine code

- Expired token with fresh authority → refuse.
- Unexpired token with stale authority (beyond profile max) → refuse.
- Unexpired 7 d Relaxed token, 7 d+ since last authority verify → refuse.
- Strict uplink grant past 1 h even if Relaxed profile would allow → refuse.
- Firmware grant past 15 min or with mismatched digest → refuse.
- Ownership/issuer/recovery grant past 5 min or non-owner holder → refuse.
- Unlimited diagnostic used for a configuration operation → refuse.
- `AuthorizerBuilder::time()` or token-asserted time as the only evidence → refuse.
- Clock moved backwards after expiry → still refuse.
- Authority verify under partition, Relaxed: issuer revoked on a reachable
  replica, destination has not verified within 7 d → refuse (extends F-D).
- Same, Standard: not verified within 24 h → refuse.
- Same, Strict: not verified within 15 min → refuse.
- Positive F-D control: issuer revoked after the destination's last verify,
  request still inside the stale window, token unexpired → accept. Tests
  MUST distinguish "refused because stale" from "accepted inside bounded
  exposure".
- No-expiry owner diagnostic with missing time evidence → refuse.
- No-expiry owner diagnostic past issuance-time stale maximum → refuse.

## Review gate

This pin needs independent instrument review (other family than the author)
before `mjolnir-admin-capabilities` grows an authorization/time API.
The closed trust-contract review does not accept these numbers.
