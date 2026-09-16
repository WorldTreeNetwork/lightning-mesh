## ADDED Requirements

### Requirement: Discoverable is not trusted

A Lightning Mesh node SHALL be able to find another Lightning Mesh
node from RF without a pre-shared `list peer` entry. Association on
the 802.11s backhaul SHALL NOT grant babel adjacency, production CRDT
write access, subnet-claim merge, or overlay SSH. Possession of the
Lightning Mesh binary SHALL NOT constitute membership.

#### Scenario: Factory node hears an island

- GIVEN a factory node with empty roster and empty addrbook
- AND an existing island beaconing a compatible capability descriptor
- WHEN the factory node is in RF range long enough to scan and associate
- THEN it learns that a Lightning Mesh island is present
- AND it does not announce babel routes to that island
- AND existing members do not accept its subnet claims

#### Scenario: Running the firmware is not membership

- GIVEN a stranger flashing official Lightning Mesh onto a Cudy box
- WHEN that box associates to `mjolnir-mesh`
- THEN it remains unauthorized until an enrollment grant exists

### Requirement: Unknown identities are quarantined

An identity that has associated but holds no capability grant SHALL
be limited to a nonce-bound iroh handshake and a rate-limited
enrollment lane. The system SHALL NOT form babel adjacency with it,
SHALL NOT merge its production CRDT writes or subnet claims, and
SHALL NOT accept overlay SSH from it.

#### Scenario: Associated but unauthorized

- GIVEN node B associated to node A's 802.11s mesh
- AND B has no endorsement or capability grant accepted by A
- WHEN B sends babel hellos, a subnet claim, a CRDT lane write, or SSH to A's `10.254`
- THEN A ignores or refuses those
- AND B can still complete the enrollment handshake

#### Scenario: Grant lifts quarantine

- GIVEN B was quarantined
- WHEN an authorized member records a valid capability grant for B
- THEN B may participate in babel and production CRDT according to that grant

### Requirement: Capability beacon is a locator

A node SHALL advertise a signed capability descriptor so a scanner can
learn protocol generation, node id, channel plan, and bootstrap
coordinates before merging. The descriptor SHALL NOT be treated as
proof of freshness or authorization. A captured beacon replayed later
SHALL NOT by itself lift quarantine.

#### Scenario: Scan before merge

- GIVEN an island advertising a signed descriptor
- WHEN a factory node scans
- THEN it can classify the island as compatible or incompatible without
  merging client L2

#### Scenario: Replay is not a ticket

- GIVEN an attacker replays a previously captured valid beacon
- WHEN a node verifies the descriptor
- THEN it may use it as a locator
- AND it still requires a nonce-bound handshake before any grant

### Requirement: Enrollment is an offer plus physical QR

A compatible untrusted node SHALL receive an enrollment offer on the
quarantine lane. An operator SHALL still be able to enroll by scanning
the new node's QR on an existing member (`met`). Threshold
countersignatures and revocation SHALL apply to both paths. Completing
the offer or QR SHALL NOT skip K-of-N or revocation. Acceptance is each
device's local policy, not a mesh-wide instant.

#### Scenario: Beacon offer

- GIVEN a quarantined compatible node
- WHEN an authorized member accepts the enrollment offer
- THEN the subject is entered into the membership CRDT under the same
  endorsement rules as QR enrollment

#### Scenario: QR still works

- GIVEN an operator with a trusted member
- WHEN they scan the new node's EnrollmentTicket QR
- THEN enrollment proceeds as in `met` even if the beacon offer was ignored

### Requirement: Babel neighbor sessions are authenticated

Babel updates used for mesh routing SHALL be bound to an authenticated
neighbor session with freshness **and** to the announcer's node
identity. Neighbor admission or session keys SHALL be per node
identity: revoking one identity SHALL NOT require rekeying other
nodes. A single fleet-wide babeld HMAC key SHALL NOT satisfy this
requirement. Prefix binding SHALL be on the update's router-id origin, not on
the immediate neighbor (jump nodes MUST re-announce prefixes behind
them). A router-id SHALL only originate prefixes it is authorized to
claim, **except** mobility host `/32`s (sz9, proto 158): a `/32` in
the mesh client space originated by any enrolled router-id with a live
session SHALL still install. `0.0.0.0/0` SHALL require a gateway grant.
An enrolled neighbor forging another member's router-id, or hijacking a
single client `/32`, is residual, handled by revocation (R5), not by
this requirement. This change does not owe a signed mobility lane.
Origin-id stamping without session freshness SHALL NOT satisfy this
requirement.

#### Scenario: Stranger cannot steal the default

- GIVEN an unauthorized or session-less neighbor
- WHEN it announces `0.0.0.0/0` at a low metric
- THEN other nodes do not install that route

#### Scenario: Replay fails closed

- GIVEN a previously valid authenticated babel update
- WHEN it is replayed after the session nonce or window has moved
- THEN it is not applied

#### Scenario: Enrolled origin cannot announce a prefix it does not own

- GIVEN node E is enrolled and has a session
- AND E's authorized claim is `10.42.12.0/24`, not the default route
- WHEN E originates `10.42.99.0/24` or `0.0.0.0/0` under E's own router-id
- THEN other nodes do not install that route
- AND a jump node re-announcing V's authorized `10.42.12.0/24` with V's
  router-id still installs

### Requirement: CRDT writes are identity-authorized

Production CRDT records for subnet claims, address book, services, and
name-lane writes SHALL carry an Ed25519 signature by the **subject**
identity over the canonical record (the same shape as leased-name
claims). Every production mutation, including release, tombstone, and
unpublish, SHALL be subject-signed over canonical bytes that include
the lane key and the HLC. Merge SHALL verify that signature **and**
that the subject holds a grant for that lane. Checking only the
delivering hop's grant SHALL NOT satisfy this requirement. HLC
first-writer-wins SHALL apply only among identities that pass both
checks.

The coordinate lane is a named carve-out: the stamper may differ from
the subject; the stamper SHALL sign and SHALL hold the stamp grant.

#### Scenario: Unauthorized claim is ignored

- GIVEN node S has no claim-write grant
- WHEN S gossips a subnet claim for a `10.42.0.0/16` slice
- THEN members do not install or persist that claim

#### Scenario: Enrolled member cannot forge another identity's record

- GIVEN node E is enrolled and holds a claim-write grant for itself
- WHEN E gossips a subnet claim or addr-book entry whose subject is node V
- THEN members reject it (signature is not V's, or V did not grant E)

#### Scenario: Enrolled member cannot release or tombstone another identity's claim

- GIVEN node V holds an authorized, subject-signed claim for `10.42.12.0/24`
- AND node E is enrolled
- WHEN E gossips a `SubnetClaimRelease` (or tombstone / unpublish) for that lane
- THEN members do not drop V's claim

#### Scenario: Authorized claim still FWW among grantees

- GIVEN two identities both hold claim-write grants and sign their own records
- WHEN they conflict on a prefix
- THEN existing HLC first-writer-wins among those identities still applies

### Requirement: Open control plane waits on three gates

The system SHALL treat an open (unencrypted, join-anyone) 802.11s
backhaul as a hostile underlay. It SHALL NOT carry trusted babel,
production CRDT, or overlay management until quarantine, authenticated
babel sessions, and identity-authorized CRDT writes are all in effect.
Until those gates exist, association SHALL NOT be documented or
implemented as membership. The live fleet's empty `MESH_KEY` is a
current fact; this change does not owe flipping it. `docs/join/node/03-join-the-mesh.md`
SHALL state that the membership gate is absent until the three gates
land. Trusted-fleet gossip bootstrap remains inventory `list peer`
(`m4a`), not RF association.

#### Scenario: Association is not a control-plane peer

- GIVEN the three gates are not all in effect
- WHEN a new node associates on open `mjolnir-mesh`
- THEN operators are not told it has joined the control plane
- AND this change does not add RF neighbors to `list peer`

### Requirement: Trusted-fleet bootstrap uses inventory peer sets

`install-node.sh` and `update-fleet.sh` SHALL render each node's
`list peer` as the full set of other node ids from `fleet-nodes.conf`
(minus self). They SHALL NOT append 802.11s station MACs or unknown
mesh neighbors. A chain of single peers SHALL NOT be the provisioned
shape.

#### Scenario: Fresh install gets everyone else

- GIVEN `fleet-nodes.conf` lists N inventory nodes
- WHEN `install-node.sh` applies on a new box whose id is then added
- THEN that box lists the other N ids
- AND each existing box can be updated to list the new id
- AND no `iw station dump` entry is copied into UCI

#### Scenario: Chain is not provisioned

- GIVEN a previous hand config with one peer per node
- WHEN update-fleet peer render runs
- THEN each node has the full inventory set, not a single next-hop id

### Requirement: Overlay management is not on the untrusted underlay

Dropbear on overlay `10.254` and mesh-zone SSH SHALL NOT accept
connections from quarantined or unauthorized identities. WAN-LAN admin
(WPS window) and on-link link-local SSH remain the physical-presence
paths they already are.

#### Scenario: Quarantined neighbor cannot SSH overlay

- GIVEN B is quarantined on A's radio
- WHEN B connects to TCP/22 on A's `10.254` address
- THEN A does not grant a session to B's unauthorized key or source

### Requirement: Untrusted or incompatible peers degrade to isolated L3

When a heard mesh is incompatible or remains untrusted, the system
SHALL NOT merge client L2. It MAY offer isolated L3/NAT interoperability.
That path SHALL NOT expose overlay SSH, production CRDT, or unrestricted
relay service.

#### Scenario: Foreign or untrusted mesh

- GIVEN a scanned descriptor that is incompatible or untrusted
- WHEN the local node chooses not to enroll
- THEN client broadcast domains stay separate
- AND any L3 forwarding is NAT'd and does not pass `10.254` management
