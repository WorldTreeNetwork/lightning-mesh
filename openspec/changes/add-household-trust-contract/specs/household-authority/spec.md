## ADDED Requirements

### Requirement: Separate bounded recovery and administrative lifetimes
The recovery ceremony SHALL default to a configurable 120-second physical window
and close on successful recovery, reboot or loss of trustworthy window timing.
Configuration-changing admin grants SHALL follow the owner's selected security
profile. The profiles SHALL be (token default / token ceiling / stale-authority
maximum):
- **Strict:** 15 minutes / 1 hour / 15 minutes
- **Standard:** 24 hours / 7 days / 24 hours. Standard SHALL be the default.
- **Relaxed:** 7 days / 30 days / 7 days. Relaxed SHALL be opt-in and SHALL
  show a persistent risk warning.

Owners MAY shorten either timer.

Regardless of remaining token lifetime, a grant SHALL be usable only while the
destination has verified fresh authority and revocation information within the
profile's stale-authority maximum. Beyond that, privileged use SHALL require
owner reauthorization. Freshness SHALL mean locally verifiable signed authority
state, not internet connectivity. Profile and policy changes SHALL apply only to
newly issued grants and SHALL NOT lengthen issued grants. Renewal SHALL require
fresh authorization checks.

Grant classes SHALL cap lifetimes regardless of profile:
- Primary Wi-Fi, backhaul, radio, routing, firewall, DNS and uplink changes: at
  most 24 hours (1 hour under Strict), with stale authority never exceeding
  24 hours.
- Firmware or software installation: a single transaction of at most 15
  minutes, bound to the artifact digest.
- Ownership, issuer, recovery-policy and protected-key changes: owner-only,
  single-use, at most 5 minutes.

A grant spanning classes SHALL take the shortest applicable cap. No
configuration-changing or delegated Admin grant SHALL be unlimited. Only private
read-only diagnostics on the owner's registered holder-bound device MAY have no
wall-clock expiry, and such grants SHALL still end on holder revocation or
authority-epoch change. Neither window SHALL replace required credential proof
or gate ordinary internet access.

#### Scenario: Standard default on an isolated router
- GIVEN a new household on the default Standard profile and an Admin grant issued with a 7-day lifetime
- WHEN a router has not verified fresh authority or revocation information for more than 24 hours
- THEN privileged changes on that router are refused until the owner reauthorizes, even though the token has not expired

#### Scenario: Relaxed profile warns about exposure
- GIVEN an owner switching the household to Relaxed
- WHEN the switch is confirmed
- THEN the owner is warned that a removed administrator may retain control of an isolated router for up to 7 days, the risk indicator stays visible, and existing grants keep their original lifetimes

#### Scenario: High-risk class ignores profile length
- GIVEN a household on Relaxed
- WHEN a firmware installation grant is requested with a 7-day lifetime
- THEN it is issued, if at all, as a single-transaction grant of at most 15 minutes bound to the artifact digest

#### Scenario: No-expiry diagnostics end on epoch change
- GIVEN a no-expiry read-only diagnostics grant on the owner's registered device
- WHEN the owner rotates the household authority epoch
- THEN the grant stops authorizing on every router that has verified the new epoch

#### Scenario: Unlimited mutation is refused
- GIVEN any profile
- WHEN an owner asks for a configuration-changing or delegated Admin grant with no expiry
- THEN issuance is refused

#### Scenario: Activity does not renew a grant
- GIVEN a Strict-profile configuration grant with a 15-minute lifetime
- WHEN the holder continues using it beyond expiry without fresh authorization
- THEN privileged changes are refused while ordinary connectivity remains available

#### Scenario: Recovery window is consumed
- GIVEN a valid enrolled credential and a fresh physically opened recovery window
- WHEN recovery succeeds or the node reboots
- THEN the window closes and cannot authorize another recovery

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
The system SHALL require both a previously enrolled recovery credential and fresh
physical presence at a node for non-destructive owner recovery. Physical
confirmation SHALL be bound to the recovery request, node identity and ownership
epoch; neither credential possession nor physical access alone SHALL suffice.
A local reset without recovery authority SHALL
NOT retain access to the former house's authority or secrets. HTTP hub code SHALL
NOT receive protected owner keys or reusable administrative bearer grants.

#### Scenario: Button on an owned node
- GIVEN a node already bound to a house
- WHEN a nearby person presses its setup button without recovery authority
- THEN they cannot replace the owner or gain the previous house's protected access

#### Scenario: Recovery credential without physical access
- GIVEN a valid previously enrolled recovery credential
- WHEN its holder requests owner recovery remotely without fresh physical confirmation
- THEN recovery is refused and existing ownership remains unchanged

#### Scenario: Both recovery factors
- GIVEN a valid enrolled recovery credential and fresh physical confirmation at a node
- WHEN both verify against the same recovery request and current ownership epoch
- THEN recovery may proceed under the authority-rotation and per-node acknowledgement rules
