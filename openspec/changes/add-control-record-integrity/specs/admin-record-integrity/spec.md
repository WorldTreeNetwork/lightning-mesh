## ADDED Requirements

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
sequence does not match, including a replay of an already-applied record and a
record made valid only by moving the clock backwards.

#### Scenario: Replayed gossip
- GIVEN a valid owner-signed record that this consumer already applied
- WHEN the same bytes are gossiped again, or an older sequence for that head arrives
- THEN the consumer rejects them and does not treat the replay as a new decision

### Requirement: Withdrawn issuers and lost freshness deny
A verified issuer withdrawal SHALL make later records from that issuer unusable
on every node that has verified the withdrawal, even when a grant has not
expired. A node SHALL deny privileged use of owner-signed records when its
freshness state is missing, corrupt, or unreadable. A node that has not seen
a withdrawal SHALL deny privileged use once the household stale-authority
maximum has elapsed.

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
refuse a set of competing heads instead of picking one. Creating a successor
head is out of this requirement.

#### Scenario: Two partition heads
- GIVEN two owner-signed heads that do not name each other
- WHEN a consumer tries to use either as the current house authority
- THEN both are refused until a single successor record exists

### Requirement: One canonical encoder
Canonical record bytes SHALL be produced and checked with the
mesh-admin-capabilities encoder and a distinct record-type domain tag. This
capability SHALL NOT define a second canonicalization.

#### Scenario: Divergent encoding
- GIVEN the same fields encoded by a second, local JSON canonicalizer
- WHEN the consumer verifies them with the mesh-admin-capabilities encoder
- THEN verification fails
