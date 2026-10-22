# add-control-record-integrity — fresh advise r2 (after the second 2026-10-22 send-back)

> **ADVISE:** accept
> **READER:** opus-arch-review
> **SPAWN:** /home/dorje/work/WorldTree/lightning-mesh/.spawns/mjolnir-mesh-ai0.9-1792654637-92929-92ec8643

## Blind take (written before opening design.md, tasks.md, the spec delta or the prior reviews)

Sources: `proposal.md`, the household-authority and mesh-admin-capabilities change specs, and `crdt/leased_name.rs`.

1. Pin: one verify predicate that every consumer runs at the point of use. It covers the signature over the mesh-admin-capabilities canonical bytes, with a per-record-type domain. Trust set at ingest is never carried along.
2. Pin: the fact class is a type, not a comment. Leased names sign only `challenge/name/port`, so `ip`, `scheme`, `app` and both HLCs are relay-forgeable. They stay node-vouched for good.
3. Pin: freshness (the stale-authority clock) advances only when an owner-signed record carrying *authority and revocation state* is verified. Gossip receipt, node-vouched records, client-key sigs and unrelated owner records do not advance it.
4. Pin: restrictive records (withdrawals, revocations) are accepted at any age. Only permissive records age out. Otherwise a late node fails open.
5. Pin: replay protection uses a signed per-signer sequence plus a persisted high-water mark. Losing or corrupting that state denies. It is never reset to empty, which is what `load_leased_names` does.
6. Pin: "now" for staleness comes from authenticated time evidence, never from an HLC `wall_clock` the record supplies.
7. Pin: same-epoch competing heads fail closed, with no HLC or LWW tiebreak.
8. Refuse: a second canonicalizer, alias resolution records, head writing, and any record that asserts its own issuer is approved.
9. Open: is a withdrawal retroactive to records the issuer signed before it? I would make it retroactive.
10. Tradeoff: checking at every consumer costs per-use Ed25519 work on routers. A per-node cache keyed by record digest is fine as long as the cache is never gossiped. Failing closed costs admin availability on isolated nodes but not connectivity.

## Comparison

I opened the r1 and r2 reviews, `design.md`, `tasks.md` and the spec delta after writing the take. To check the seams I also read `add-mesh-admin-capabilities/time-evidence.md` and `codec-profile.md`, `crates/mjolnir-admin-capabilities/src/engine.rs`, and `add-household-owner-claim/design.md`.

| Take | Design / spec | Match |
|---|---|---|
| 1 verify at consumer | Pin 2; requirement "Consumers verify" | Yes |
| 2 vouched by type; leased names | Pin 1; Leased names (N2) section | Yes |
| 3 freshness only on authority-state | Freshness and replay section; scenario "Withdrawn issuer keeps signing" | **Yes (R1(a))**. Spec body wording lags; see N1. Head content is open; see N2. |
| 4 restrictive records never age out | Withdrawal (B3) section; scenario "Old withdrawal still narrows" | **Yes (R1(b))** |
| 5 sequence + persisted high-water | Sequence and constructors (B2) section | Yes |
| 6 authenticated "now" | "judges signed time with `TimeEvidence` … not the wall clock" | Yes (r2 N-d closed) |
| 7 same-epoch fork fails closed | Fork resolution (B4) section | Yes. Epoch-max consequence is stated only implicitly; see N4. |
| 8 scope refusals | Not owed here; Encoder (N1) section | Yes |
| 9 withdrawal retroactive | "regardless of the record's signing time" | Yes |
| 10 tradeoff | Diagnostics and forwarding continue; closed deny-reason set | Yes (r2 N-c closed) |

### The two gating questions

- **R1(a), freshness advances only on owner-signed authority-state records.**
  Answered. design.md: "Only an owner-signed authority-state record advances
  freshness … Records from non-owner issuers, and owner-signed records of any
  other type, do not advance freshness. Receipt time never advances
  freshness." The spec delta adds the partition scenario r2 asked for, almost
  verbatim. A relay that forwards unrelated owner records while withholding a
  withdrawal can no longer keep a node fresh. This matches household-authority's
  "Freshness SHALL mean locally verifiable signed authority state".
- **R1(b), withdrawals are accepted at any age.** Answered. design.md: "The
  5-minute ownership-class cap bounds the owner's authorization to sign that
  withdrawal. It is not a lifetime of the withdrawal record. Withdrawals and
  revocations are accepted at any age … Only permissive records are refused
  for staleness." The spec scenario "Old withdrawal still narrows" covers the
  case of a withdrawal seen after both 5 minutes and the stale maximum.

The r2 non-blockers are also closed. N-a: the tasks.md verifier line now
requires `VerifiedAuthority` to be constructable only from this verifier.
`engine.rs:148` still has all-`pub` fields today, so that task is real work.
N-c: there is now a closed deny-reason set. N-d: `TimeEvidence` is named.

### Steelman against my take

My take 10 worries about per-use verification cost. The design does not
address it and does not need to. Ed25519 verification is cheap next to a
config apply, and a local digest cache does not change the contract. I
withdraw it as a concern.

My take 3 is stricter than the design in one respect. I require the
freshness-advancing record to *carry* revocation state. The design lists
"the current head" as an authority-state record alongside a snapshot that
states the complete issuer and revocation set. The strongest case for the
author is that heads are rare, owner-written, and advance the authority
epoch, and that household-authority already ends grants on epoch change. A
head would then implicitly restate authority. That holds only if issuer
approvals are epoch-scoped, which neither this change nor
`add-household-owner-claim` states. I record this as N2. It is not
blocking: it is a schema-content decision for the first unchecked task,
and the R1 rule ("only authority-state records") is correct as written.

## Non-blocking

All of these should be fixed before fold. They are wording and schema
details, and none of them reopens R1.

**N1. The spec requirement body still states the pre-R1 freshness rule.**
"Forged and replayed records fail closed" says: "Freshness SHALL advance only
to that signed time for a newly verified newer record". Read alone, that
admits *any* newly verified record. Only the scenario narrows it. Because the
requirement body is what folds into the living spec, it should say
"…for a newly verified newer owner-signed authority-state record (the
current head or an issuer/revocation snapshot chained to it)". Similarly, the
design's B1 sentence "A cold node rejects an old record by that signed time"
should read "an old *permissive* record", to match the B3 section. The spec
should also carry "withdrawals and revocations SHALL be accepted at any age"
as SHALL text, not only as a scenario.

**N2. Pin what a head must contain to advance freshness.** Take a withdrawal
W signed at epoch E, followed by a new head H at E+1 (for example, a
recovery). If H does not state or commit to the issuer and revocation set,
and issuer approvals carry across epochs, then a relay can forward H and
withhold W. The node becomes fresh while still trusting the withdrawn issuer.
Close this in the record-schema task in one of two ways:
- every freshness-advancing record, the head included, carries or
  digest-commits to the complete issuer and revocation set, or
- issuer approvals and revocations are epoch-scoped, so an epoch advance
  empties the issuer set until owner-signed records restate it at E+1.

`add-household-owner-claim` writes the head and should be told which one applies.

**N3. Withdrawal vs. later re-approval ordering.** B3 implies that the owner
can un-withdraw an issuer. Accepting withdrawals at any age means an old W
that arrives after a newer owner re-approval A must not re-withdraw the
issuer. Per-type sequences cannot order W against A, because they belong to
different record types. Order them by the authority-state snapshot, or by
signed `issued_at` under the owner signer. This only affects availability:
the failure direction is safe.

**N4. State that the fork rule is epoch-max.** This carries over from r2 N-b.
"No tiebreak" holds only among same-epoch siblings. A higher-epoch branch wins
automatically, so whoever holds a stolen owner key can out-epoch a
recovery. Say this once in design.md so that `add-household-owner-claim`'s
recovery pin is written with that in mind.

**N5. `codec-profile.md` does not list the record domains yet.** The design
says that codec-profile.md "records the new domains". Today it lists only
`lightning-admin/request` and `lightning-admin/token/v1`. Fold the
amendment into the record-schema task.

## Verdict

R1(a) and R1(b) are answered in design.md and pinned by dedicated spec
scenarios. The tasks.md verifier line carries both rules into
implementation. Accept. I have checked the "Fresh advise after preparation"
box in tasks.md. N1–N5 can be edits or beads. N1 and N2 should land before
fold.
