# Control-record integrity

## Pins

1. **Two fact classes.** `node-vouched` is a self-report the ingesting node
   sanity-checked (address, radio observation, p43/71x leased-name ingest).
   `owner-signed` is a record whose canonical bytes verify under an owner or
   issuer key. Control, membership, issuer pin, and authority-epoch facts are
   owner-signed. A vouched fact never satisfies an owner-signed check.

2. **Verify at the consumer.** The node that acts on the record checks the
   signature, domain tag, house, epoch, and canonical bytes. A peer flag
   "already verified" is not evidence.

3. **One encoder.** Canonical bytes come from the `mjolnir-admin-capabilities`
   encoder (`add-mesh-admin-capabilities`). This change passes a distinct
   record-type domain tag into that encoder. It does not add a second JSON
   canonicalizer. If the encoder cannot take a domain tag yet, extend that
   encoder; do not fork it.

4. **Replay.** The signed bytes include a house authority epoch and a
   per-head sequence. An older sequence for the same head, or a byte-identical
   record already applied, is rejected. Clock rollback does not revive it.

5. **Withdrawn issuer.** A verified withdrawal makes later records from that
   issuer unusable on nodes that have the withdrawal, including grants that
   have not expired. A node that has not seen the withdrawal denies privileged
   use once the household stale-authority maximum has elapsed (Standard: 24
   hours). Silence is not continued trust past that bound. Unreadable or
   corrupt freshness state denies immediately.

6. **One chain.** An authority-transition record names the previous head.
   Gossip must not merge two competing heads. Consumers refuse a fork until
   a single successor exists. This change verifies that rule.
   `add-household-owner-claim` is the only writer of a new head.

## Freshness and replay (advise 2026-10-22 B1)

Every owner-signed record carries issuer-signed `issued_at` and, where the
record expires, `expires_at`, inside the canonical bytes. Only an owner-signed
authority-state record advances freshness: the current head, or an
owner-signed issuer and revocation snapshot chained to it, stating the
complete issuer set and revocations as of its `issued_at`. Records from
non-owner issuers, and owner-signed records of any other type, do not advance
freshness. Receipt time never advances freshness. A cold node judges signed
time with `TimeEvidence` from mesh-admin-capabilities, not the wall clock.
Re-receiving an already-applied record, or an older one, leaves freshness
unchanged. A cold node rejects an old record by that signed time against the
household stale-authority maximum. Replaying the newest valid record after
that maximum does not restore privileged use.

## Sequence and constructors (B2)

The sequence scope is `(house, epoch, signer, record-type, subject)`. Signers
do not share one counter. Re-gossip of an already-applied record is a no-op:
it does not demote applied state and it does not count as a new decision.
The applied set and high-water marks are freshness state. Missing or corrupt
freshness state denies. It is never replaced with an empty book.

`OwnerVerified` from this verifier is the only production constructor for
`VerifiedAuthority` and for `TimeEvidence.authority_verified_at` in
`mjolnir-admin-capabilities`. Callers cannot assemble those values from a
node-vouched fact.

## Withdrawal (B3)

Once a withdrawal verifies, no record from that issuer authorizes control,
regardless of the record's signing time. Only the owner signs a withdrawal.
The 5-minute ownership-class cap bounds the owner's authorization to sign
that withdrawal. It is not a lifetime of the withdrawal record. Withdrawals
and revocations are accepted at any age, because they only narrow authority.
Only permissive records are refused for staleness. Another issuer cannot
withdraw or un-withdraw a peer.

## Fork resolution (B4)

A successor at a higher epoch that names one branch supersedes every sibling
at the lower epoch. Same-epoch siblings stay refused until that happens.
A forked node does not fall back to the common ancestor for privileged use.
Read-only diagnostics and the data plane continue. Deny reasons are a closed
set: forged, replayed, withdrawn issuer, forked head, stale, freshness
unreadable, node-vouched.

## Encoder (N1)

Reuse means the existing canonical-CBOR profile and decode/re-encode gate in
`mjolnir-admin-capabilities`, with closed per-record schemas and new domain
strings `lightning-admin/record/<type>` version 1. It does not mean passing a
tag through `AdminRequest::encode`, and it does not mean a second JSON
canonicalizer. `codec-profile.md` in that change records the new domains.

## Leased names (N2)

Leased-name records stay node-vouched. Their v1 preimage does not cover
address, scheme, app, or the time fields, and a multi-hop publish does not
authenticate the originator. "Vouched by" names the delivering peer, or
nothing. This change does not upgrade p43/71x comments into owner signatures.

## Consumers (N3)

This change is the library instrument. meshd and Lightning Admin call the
crate. Admin does not reimplement the check in TypeScript.

## Not owed here

- AliasTable resolution records.
- Choosing the device-proof or button ceremony.
- Live radio or fleet SSH.
