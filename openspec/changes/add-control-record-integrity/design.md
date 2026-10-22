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

## Not owed here

- AliasTable resolution records.
- Choosing the device-proof or button ceremony.
- Live radio or fleet SSH.
