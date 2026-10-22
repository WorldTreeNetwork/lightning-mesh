# admin-record-integrity

Consumers verify owner-signed admin records themselves. Node-vouched
facts, including leased-name ingest, never count as owner authority.
This capability does not write a new authority head. Folded from
`add-control-record-integrity` (2026-10-22).

Code: `crates/mjolnir-admin-capabilities/src/record.rs`.

## Requirements

### Requirement: Consumers verify owner-signed admin records
A node SHALL use an owner-signed admin record only after it verifies the
signature, record-type domain, house, authority epoch, and canonical bytes
itself. Another node's claim that it already verified the record SHALL NOT
authorize use.

#### Scenario: Non-ingesting consumer
- GIVEN a gossip payload whose sender asserts that the record was verified
- WHEN this node has not verified the signature over the canonical bytes
- THEN it rejects the record for admin use

### Requirement: Node-vouched facts are not owner authority
Node-vouched observations, including leased-name ingest, SHALL stay marked
node-vouched. They SHALL NOT satisfy a check that requires an owner-signed fact.

#### Scenario: Vouched address used as ownership
- GIVEN a node-vouched address or name record
- WHEN a control decision requires an owner-signed fact
- THEN the decision is denied

### Requirement: Forged and replayed records fail closed
The verifier SHALL reject a record whose signature, canonical bytes, epoch, or
sequence does not match, including a record made valid only by moving the clock
backwards. Re-gossip of an already-applied record SHALL be a no-op and SHALL
NOT demote applied state or advance freshness. Every owner-signed record SHALL
carry an issuer-signed `issued_at` inside the canonical bytes. Freshness SHALL
advance only to that signed time for a newly verified newer record, never to
receipt time.

#### Scenario: Replayed gossip
- GIVEN a valid owner-signed record that this consumer already applied
- WHEN the same bytes are gossiped again, or an older sequence for that subject arrives
- THEN the consumer does not treat the replay as a new decision and does not advance freshness

#### Scenario: Newest record replayed after the stale maximum
- GIVEN a node whose last signed freshness is older than the household stale-authority maximum
- WHEN a peer replays the newest already-applied owner-signed record
- THEN privileged use stays denied

#### Scenario: Withdrawn issuer keeps signing
- GIVEN a withdrawn issuer partitioned with a node that never saw the withdrawal
- WHEN that issuer keeps publishing newly signed records that are not authority-state snapshots
- THEN privileged use is denied once the stale maximum passes since the last owner-signed authority-state record

#### Scenario: Old withdrawal still narrows
- GIVEN a withdrawal signed under a valid 5-minute owner authorization
- WHEN a node sees that withdrawal after the 5 minutes and after the stale maximum
- THEN the withdrawal still removes that issuer and does not itself get rejected for age

### Requirement: Withdrawn issuers and lost freshness deny
Once a node has verified an owner-signed issuer withdrawal, no record from
that issuer SHALL authorize control on that node, whatever the record's
signing time. Only the owner SHALL sign a withdrawal. A node SHALL deny
privileged use of owner-signed records when its freshness state is missing,
corrupt, or unreadable, and SHALL NOT replace that state with an empty book.
A node that has not seen a withdrawal SHALL deny privileged use once the
household stale-authority maximum has elapsed. Freshness state includes the
applied set and high-water marks.

#### Scenario: Withdrawal during an unexpired grant
- GIVEN a verified withdrawal of an issuer and a grant from that issuer that has not expired
- WHEN a node that has the withdrawal evaluates an admin record signed by that issuer
- THEN the record is rejected

#### Scenario: Freshness state unreadable
- GIVEN a structurally valid owner-signed record
- WHEN this node's stored freshness state cannot be read
- THEN privileged use is denied

### Requirement: A forked authority head is not merged
An authority-transition record SHALL name one previous head. Consumers SHALL
refuse a set of competing heads instead of picking one, and SHALL NOT fall
back to their common ancestor for privileged use. A successor at a higher
epoch that names one branch SHALL supersede every sibling at the lower epoch.
Read-only diagnostics and ordinary forwarding SHALL continue during a fork.

#### Scenario: Two partition heads
- GIVEN two owner-signed heads that do not name each other
- WHEN a consumer tries to use either, or their common ancestor, as the current house authority
- THEN privileged use is refused until a higher-epoch successor names one branch

#### Scenario: One successor covers the sibling
- GIVEN a higher-epoch successor that names one of two sibling heads
- WHEN a consumer that saw both siblings verifies the successor
- THEN the successor is the current head and the other sibling is not

### Requirement: One canonical encoder
Canonical record bytes SHALL use the mesh-admin-capabilities canonical-CBOR
profile, its decode/re-encode gate, and a closed schema whose domain string
is `lightning-admin/record/<type>` version 1. This capability SHALL NOT define
a second canonicalization, and SHALL NOT treat a node-vouched value as
`VerifiedAuthority`.

#### Scenario: Divergent encoding
- GIVEN the same fields encoded by a second canonicalizer
- WHEN the consumer verifies them with the mesh-admin-capabilities record schema
- THEN verification fails
