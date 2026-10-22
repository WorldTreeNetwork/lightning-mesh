# add-control-record-integrity — fresh advise (after 2026-10-22 send-back)

> **ADVISE:** send-back
> **READER:** opus-arch-review
> **SPAWN:** /home/dorje/work/WorldTree/lightning-mesh/.spawns/mjolnir-mesh-ai0.9-1792654293-88964-3b3f2437

## Blind take (written before opening design.md, tasks.md, the spec delta or the prior review)

1. Pin: each consumer verifies owner-signed records itself through the mesh-admin-capabilities encoder, with a per-record-kind domain tag; no "ingester verified" bit is carried or trusted.
2. Pin: the signature covers every field a consumer acts on, including order/freshness fields (epoch, sequence, HLC) and issuer id. Unsigned fields are node-vouched by type (separate wrapper), not by comment.
3. Pin: `leased_name.rs` signs only `challenge\nname\nport`; `ip`, `scheme`, `app`, `first_claimed_at` and `renewed_at` are unsigned, so any relay can backdate `first_claimed_at` to win reclaim or re-stamp `renewed_at`. Even after sig re-verification, these records stay node-vouched and feed no admin decision.
4. Pin: replay protection is a signed monotonic counter/epoch plus a persisted high-water mark per (issuer, kind, subject); an unreadable high-water mark denies.
5. Pin: withdrawn issuer is judged against the verified authority head, not the record's self-asserted time.
6. Pin: two validly signed heads at the same epoch fail closed. No HLC/lexicographic tie-break (the CRDT reflex is wrong here).
7. Pin: freshness reuses household-authority's per-profile stale-authority maximum; no new clock or timer.
8. Refuse: a second encoder, an ingester-set trust flag, HLC-merge of owner-signed conflicts, and absorbing alias resolution records or head writing.
9. Refuse: verifier facts taken from record content (a record asserting that its own issuer is approved).
10. Tradeoff: fail-closed on fork or stale freshness costs admin availability (not connectivity) on isolated nodes. Accept it, but deny reasons must stay distinguishable (forged / replayed / withdrawn / forked / stale) so the UI never reports "healthy". Open question: the capabilities encoder is specified for *requests*, so how does a *record* reuse it without forking it?

## Comparison

I opened the r1 review, `design.md`, `tasks.md` and the spec delta after
writing the take above. I also read `add-mesh-admin-capabilities/time-evidence.md`
and `crates/mjolnir-admin-capabilities/src/engine.rs`, because the B1 and B2
answers name `TimeEvidence` and `VerifiedAuthority`.

| Take | Design / spec | Match |
|---|---|---|
| 1 consumer verifies, no ingester bit | Pin 2, requirement "Consumers verify" | Yes |
| 2 every acted-on field signed; vouched by type | Pin 1, signed `issued_at`/`expires_at`, N2 section | Yes. `VerifiedAuthority` is still all-`pub` today; see N-a. |
| 3 leased names stay node-vouched | N2 section | Yes |
| 4 replay scope, persisted high-water mark | B2 section | Yes |
| 5 withdrawal judged against authority, not record time | B3 section | Yes for the withdrawal rule. **The freshness rule reopens the gap.** See R1. |
| 6 fork fails closed, no tiebreak | B4 section | Yes. One consequence goes unstated; see N-b. |
| 7 freshness reuses the profile maximum | B1 section, pin 5 | Yes for the bound. **Too broad on which records advance it.** See R1. |
| 8 scope refusals | "Not owed here" | Yes |
| 9 no verifier facts from record content | Pin 2, the B2 constructor pin | Yes |
| 10 deny reasons distinguishable; encoder for records | The N1 section answers the encoder. Reasons are not pinned. | Partial. See N-c. |

### r1 blockers

- **B1 (signed freshness anchor).** Answered as asked. Records carry signed
  `issued_at`. Receipt time never advances freshness. Replaying an applied
  record is a no-op. Replaying the newest record past the stale maximum does
  not restore privileged use. R1 below is a residual of B1's *purpose*: a
  relay or withdrawn issuer must not be able to keep a node fresh.
- **B2 (sequence scope, re-gossip, never-empty, seam).** Answered. The scope
  is `(house, epoch, signer, record-type, subject)`. Re-gossip is a no-op
  that never demotes applied state. Freshness state is never reset to empty.
  `OwnerVerified` is the only production constructor.
- **B3 (retroactivity, who withdraws).** Answered. Withdrawal is retroactive
  and owner-only. See R1(b) for the 5-minute wording.
- **B4 (way out of a fork).** Answered with r1 option (a): a higher-epoch
  successor that names one branch supersedes its lower-epoch siblings. The
  node does not fall back to the common ancestor. Diagnostics and forwarding
  continue.

### Steelman

The author's strongest case is that `time-evidence.md` already defines
`authority_verified_at` as "last successful authority/revocation verify", so
the engine scopes freshness and this design does not need to. That fails
because the design names this verifier as the *only production constructor*
of `TimeEvidence.authority_verified_at`, and sets its rule as "the signed
`issued_at` of a record that verifies and is newer than the applied record
for that subject". That covers any record type, any subject and any signer.
This change's rule is the one that takes effect, and it is broader than
the engine contract it feeds.

A second argument is that household-authority already accepts exposure from
an isolated node ("a removed administrator may retain control of an isolated
router for up to 7 days"). That bound is the point. Under the current
wording the exposure has no bound at all.

## Blocking (send-back)

**R1. Freshness can be advanced by a record that carries no authority or
revocation state (a B1 residual).**

(a) *Which records advance freshness.* The design says every owner-signed record carries an
"issuer-signed `issued_at`", and any newly verified newer record advances
freshness. Take issuer I, withdrawn by the owner, partitioned together with
node N, or reaching N through a relay that drops the withdrawal. I keeps
signing fresh records for any subject. Each record verifies (N has no
withdrawal), each is newer, and each advances `authority_verified_at`. N
never reaches the stale maximum, so I keeps control indefinitely. A relay
can do the same thing without I's help. It forwards newer owner-signed
records for unrelated subjects, such as a Wi-Fi setting, and withholds only
the withdrawal. Household-authority requires "verified fresh authority
**and revocation** information". A record about an unrelated subject
carries neither.

Pin: only an **owner-signed authority-state record** advances freshness.
That is the current head, or an owner-signed issuer/revocation snapshot
chained to it. It states the complete issuer set and revocations as of its
signed `issued_at`. Records from non-owner issuers, and owner-signed records
of any other type, never advance freshness. A newer snapshot then implies
every withdrawal before it, so withholding one withdrawal while forwarding
newer state is impossible. Add a spec scenario: "GIVEN a withdrawn issuer
partitioned with a node that never saw the withdrawal, WHEN the issuer keeps
publishing newly signed records, THEN privileged use is denied once the
stale maximum passes since the last owner-signed authority-state record."

(b) *Restrictive records never age out.* The B1 section says "a cold node
rejects an old record by that signed time against the stale maximum". The
B3 section says a withdrawal "uses the ownership-class cap (at most 5
minutes)". Together they read as: a withdrawal more than 5 minutes old, or
older than the stale maximum, is rejected on arrival. That fails open for
the late node. Pin that the 5-minute single-use cap bounds the owner's
*authorization to sign* a withdrawal, not the withdrawal record. Also pin
that withdrawals and revocations are accepted at any age, because they only
narrow authority. Only permissive records are refused for staleness.

Two or three sentences in design.md plus one scenario close R1. No
restructuring is needed.

## Non-blocking

**N-a. Name the constructor task.** `VerifiedAuthority`
(`crates/mjolnir-admin-capabilities/src/engine.rs:148`) still has all-`pub`
fields. The design makes `OwnerVerified` the only production constructor,
but no task says to close the fields, for example with private fields plus
a `pub(crate)` or sealed constructor. Add that to the "Consumer verifier"
task line. Otherwise the pin remains policy rather than structure.

**N-b. State that the fork rule is epoch-max.** The B4 rule resolves forks
of unequal depth automatically. Take branch C extended to C′ at epoch E+2
and a sibling D at E+1. C′ supersedes D with no resolving act by the owner.
Only owners write heads, so this is defensible. Still, "no tiebreak" holds
only for same-epoch siblings. The design should say so explicitly, so that
`add-household-owner-claim` and the recovery work see the consequence: a
holder of a stolen owner key can outrun a recovery by out-epoching it.

**N-c. Distinguishable deny reasons.** The user journey promises that lost
freshness "is reported as denied, not as a healthy admin state".
mesh-admin-capabilities already has a reauthorization-required reason. Pin
a closed reason set (forged / replayed / withdrawn issuer / forked head /
stale / freshness unreadable / node-vouched), so consumers can show which
one applies without parsing strings.

**N-d. "Now" for a cold node.** The phrase "a cold node rejects an old
record by that signed time" depends on `TimeEvidence.unix_seconds` from
ai0.1: the persisted floor plus a monotonic reading, or a challenge-bound
attestation. Point there in one clause, so nobody implements it against the
node wall clock.

## What would flip to accept

Close R1(a) and R1(b) in design.md, and add the withdrawn-issuer partition
scenario to the spec delta. N-a through N-d may be edits or beads.
I left the "Fresh advise after preparation" box in tasks.md unchecked.
