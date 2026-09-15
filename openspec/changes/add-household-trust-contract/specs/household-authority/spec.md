## ADDED Requirements

### Requirement: Physical claim anchors authority
The system SHALL bind one initial owner to an unclaimed node only through verified
device-specific proof and a fresh physical ceremony, atomically persisted against
the node's identity and ownership epoch. Developer SSH access and discovery SHALL
NOT constitute consumer ownership.

#### Scenario: Competing claim
- GIVEN an unclaimed node with one fresh physically armed ceremony
- WHEN two different owner credentials race to claim it
- THEN at most one durable binding succeeds and retries cannot overwrite it

### Requirement: Destination enforces bounded agency
The destination SHALL verify trusted issuer state, complete Biscuit checks, exact
house and node audience, operation scope, holder signature, epochs, expiry and
freshness before authorizing control. Role labels and membership SHALL NOT grant
administration. Verifier facts SHALL NOT be replaceable by token assertions.

#### Scenario: Narrower grant
- GIVEN an approved issuer's holder-bound grant for Wi-Fi management
- WHEN the holder attenuates it to diagnostics on one node before expiry
- THEN matching diagnostics are allowed and configuration or other targets refused

#### Scenario: Changed holder or issuer
- GIVEN a valid grant for Alice
- WHEN Bob presents it with his signature, or substitutes an unapproved issuer
- THEN the destination refuses even if public directory records label Bob an admin

### Requirement: Delegation preserves ceilings
The system SHALL treat holder-changing delegation as authorized bounded reissuance,
not holder replacement through attenuation. Reissuance SHALL preserve the parent's
delegable scope, expiry ceiling and revocation lineage.

#### Scenario: Attempted expansion
- GIVEN a grant with no delegated issuance authority
- WHEN its holder requests a grant for another person
- THEN issuance is refused without an independently authorized owner approval

### Requirement: Durable freshness and honest revocation
The destination SHALL serialize request acceptance against replay and authority
changes, persist freshness state, and refuse privileged requests when grant time
validity cannot be established. The controller SHALL distinguish acknowledged
revocation from enforcement still pending on offline nodes.

#### Scenario: Lost reply and restart
- GIVEN an accepted request whose reply was lost
- WHEN the same signed request is retried after restart
- THEN it is not executed again and a recorded or explicit uncertain outcome returns

#### Scenario: Clock rollback
- GIVEN a grant whose valid lifetime has elapsed
- WHEN the node clock moves backwards or loses trustworthy time
- THEN the old grant is not made valid again

### Requirement: Recovery does not create a takeover path
The system SHALL require existing owner-rooted or previously enrolled recovery
authority for non-destructive ownership recovery. A local reset without it SHALL
NOT retain access to the former house's authority or secrets. HTTP hub code SHALL
NOT receive protected owner keys or reusable administrative bearer grants.

#### Scenario: Button on an owned node
- GIVEN a node already bound to a house
- WHEN a nearby person presses its setup button without recovery authority
- THEN they cannot replace the owner or gain the previous house's protected access
