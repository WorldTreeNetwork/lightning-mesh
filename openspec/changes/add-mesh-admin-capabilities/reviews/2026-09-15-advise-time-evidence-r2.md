# Advise r2: ai0.1.2 time-evidence pin after send-back T-1..T-5

> **ADVISE:** accept
> **READER:** fable-5.1-arch-review
> **SPAWN:** /home/dorje/work/WorldTree/lightning-mesh/.spawns/mjolnir-mesh-ai0.1.2-1789511381-300904-dff24914

## Blind take (written from Why, living spec delta, codec-profile.md §engine pins, `verify_holder_proof` only, before opening the r1 send-back or the amended pin)

1. Pin: time evidence is a distinct verifier-side input type, never `SystemTime::now()` or `AuthorizerBuilder::time()`; the canonical request carries `authority_epoch` and no timestamp, so "now" must come from outside the request and the token.
2. Pin: every absolute comparison needs two trusted points, a trusted now and a trusted anchor (issued_at); "valid at most 24 h" is undefined without the anchor.
3. Pin: token-claimed times are issuer claims that can only shorten; the anchor lives in the issuer-signed authority block, never an appended block.
4. Refuse: any fallback to a local clock when evidence is missing; missing evidence denies.
5. Refuse: unsigned time evidence or evidence from a non-approved signer; the evidence itself ages and is bounded by the stale-authority maximum.
6. Pin: stale-authority elapsed time is measured from the last verified authority snapshot in a way a wall-clock step cannot shrink.
7. Pin: the evidence must be tied to the authority state it was verified with, so a fresh timestamp cannot be paired with an older authority view.
8. Refuse: any skew tolerance wider than the profile's stale-authority maximum; skew only ever extends deny.
9. Tradeoff: RTC-less routers boot at 1970; fail-closed until an authority contact re-anchors is right even though Strict households will see denials after a power cut.
10. Author must answer: where issued_at comes from, and how "no evidence" is distinguished from "evidence too old".

## Scope

Targeted re-review of `time-evidence.md` at commit `5ae53b1` against the r1 send-back
(`2026-09-15-advise-time-evidence.md`). Diff confirmed: 50 insertions, 13 deletions,
pin file only. Profile table unchanged and still identical to `design.md` and the
`mesh-admin-capabilities` spec delta (Strict 15 min / 1 h / 15 min; Standard 24 h /
7 d / 24 h; Relaxed 7 d / 30 d / 7 d). DECIDED v2 profiles not reopened. Codec crate
still has no time symbol; `verify_holder_proof` still returns possession only.
No engine code reviewed or requested.

## T-1..T-5 verdict

| Finding | Closed? | Constraint text now in pin |
|---|---|---|
| T-1 provenance / floor / ordering | yes | `unix_seconds` from signed material plus monotonic elapsed, "SHALL NOT be filled from `SystemTime::now()` ... including after a successful authority sync"; persisted high-water floor that never decreases, below-floor evidence refused; `authority_verified_at <= unix_seconds` else refuse; staleness is only `unix_seconds - authority_verified_at`, never wall-clock subtraction; monotonic-window loss fails closed. |
| T-2 issuance-time input | yes | Authority block SHALL carry issuer-signed `issued_at` / `expires_at` as claims; missing refuses; `issued_at > unix_seconds` refuses; effective expiry `min(expires_at, issued_at + class cap, issued_at + profile ceiling, parent ceiling)`; only `unix_seconds` feeds the Datalog `time` fact. |
| T-3 profile source / which profile | yes | Profile and class are verifier inputs from verified authority state at the grant's issuance epoch; token-asserted profile accepted only if not looser; stale maximum for an issued grant is the issuance-time profile. |
| T-4 Unlimited under stale authority | yes | Unlimited waives token expiry only; stale-authority maximum and missing-time refusal still apply; partition exposure "bounded by stale-authority, not by no expiry" stated in words. |
| T-5 F-D per profile + positive control | yes | Relaxed, Standard and Strict partition vectors; positive control (revoked after last verify, inside window, unexpired → accept) with the "refused because stale" vs "accepted inside bounded exposure" distinction required. |

Acceptance contrast holds: wall clock cannot masquerade as freshness (T-1 a, b, d),
and cap arithmetic has an issuance anchor (T-2). Blind-take items 1–6, 9, 10 are
answered by the same text.

## Notes for the engine packet (non-blocking, do not edit the pin in this pass)

- **Unlimited vs "missing either refuses".** The T-2 sentence "Missing either
  refuses" is unqualified, while the Unlimited diagnostic class by definition has
  no `expires_at`. A literal engine refuses all Unlimited tokens, which is
  fail-closed, so this is not a hole. The engine packet should qualify it: absent
  `expires_at` is permitted only for the Unlimited diagnostic class; `issued_at`
  is always required.
- **Evidence-to-authority-state binding (blind take 7).** `authority_verified_at`
  is a time, not an epoch. The pin's intent is clear (it is the verify time of the
  authority state the engine evaluates against), but the engine packet should
  state that the bootstrap updates `authority_verified_at` only together with
  the authority/revocation snapshot it verified, never independently.
- **Skew.** The pin defines no skew tolerance, which reads as zero. That is the
  safe reading; the engine packet should say so explicitly rather than leave it
  to be discovered.
- **Wording.** "waives wall-clock token expiry only" under Unlimited means the
  `expires_at` check; the pin elsewhere forbids the wall clock as an input. Rename
  in a later edit to avoid a reader taking "wall-clock" as a permitted source.

None of the above reopens a T-n or changes a decided number. Engine work under
`mjolnir-mesh-ai0.1` may consume this pin.
