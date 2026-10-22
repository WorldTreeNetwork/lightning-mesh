# household-owner-claim

An unclaimed node binds one owner only when the same claim has device
proof, a card-secret HMAC, and a fresh local button window. Recovery
needs an enrolled credential and a new window together, then retires
prior recovery credentials. Forked recoveries stay unverified until one
successor names a parent. hello.mesh never receives owner secrets.
Folded from `add-household-owner-claim` (2026-10-22).

Code: `crates/mjolnir-mesh/src/owner_claim.rs`,
`crates/mjolnir-hello/src/claim_relay.rs`,
`deploy/openwrt/files/etc/rc.wps/00-mjolnir-wan-admin`.

## Requirements

### Requirement: Initial claim needs device proof, secret, and presence
The system SHALL bind one initial owner to an unclaimed node only when, for the
same claim request, the client has verified the node's meshd identity signature
against the node id on the off-device card, the claim carries an HMAC of that
request under the card secret, and a fresh local button window then confirms
that request. The node SHALL NOT display the secret. Developer SSH, discovery,
and the WPS protocol SHALL NOT constitute ownership. A local reset SHALL clear
the binding and the stored verifier and SHALL NOT re-arm the original card.

#### Scenario: Competing claim
- GIVEN an unclaimed node with one open physical window
- WHEN two different owner credentials race to claim it
- THEN at most one durable binding succeeds and a retry cannot overwrite it

#### Scenario: Secret proof without the button
- GIVEN a valid HMAC of the claim under the card secret
- WHEN no local button window confirms that request
- THEN the claim is refused

#### Scenario: Sniffed proof with a different owner key
- GIVEN an HMAC bound to the purchaser's owner public key
- WHEN a relay presents that proof with a different owner public key
- THEN the claim is refused

#### Scenario: Ceremony press does not open WAN SSH
- GIVEN an unclaimed node
- WHEN the setup button is pressed
- THEN the ceremony window arms and WAN SSH is not toggled

#### Scenario: Button without the secret
- GIVEN a fresh local button window
- WHEN the provisioning secret does not match
- THEN the claim is refused and the node stays unclaimed

### Requirement: Recovery needs both factors and retires old credentials
Non-destructive recovery SHALL require an enrolled recovery credential and a
fresh physical window bound to the same request, node id, and current ownership
epoch. Success or a completed ownership transfer SHALL retire every prior
recovery credential and require re-enrollment under the new epoch.

#### Scenario: Credential from another network
- GIVEN a valid enrolled recovery credential
- WHEN its holder requests recovery without a fresh physical window on the node
- THEN recovery is refused and the current owner remains

#### Scenario: Button on an owned node
- GIVEN a node bound to a house
- WHEN someone presses the setup button without an enrolled recovery credential
- THEN they cannot replace the owner

### Requirement: Partition recoveries do not pick a winner
The claim component SHALL persist a recovery head only for a ceremony it
verified. It SHALL NOT choose between two competing heads. A later successor
SHALL be a new verified ceremony that names one parent.

#### Scenario: Two partitions recover
- GIVEN two partitions that each completed recovery for the same house
- WHEN either head is offered as the sole authority without a successor
- THEN neither head replaces the other

### Requirement: The hub does not receive owner secrets
hello.mesh HTTP code SHALL NOT receive the owner private key, the raw
provisioning secret, or a reusable administrative bearer.

#### Scenario: Hub offers no secret entry
- GIVEN the hello.mesh page
- WHEN a purchaser opens it on an unclaimed node
- THEN the page does not accept the card secret and hub handlers do not receive it

#### Scenario: No verifier keeps the WAN toggle
- GIVEN a node with no provisioning verifier
- WHEN the setup button is pressed
- THEN the existing WAN SSH toggle runs and no ceremony window arms

### Requirement: Physical window and admin grants use different clocks
The physical window SHALL default to 120 seconds and SHALL close on success,
reboot, or loss of trustworthy timing. Five failed proofs SHALL close the
window. A later press MAY open a new window. A sniffed proof SHALL NOT
replay, because each attempt uses a fresh nonce. Configuration-changing
admin grants SHALL use the household-authority profiles and class caps,
not a separate 900 second default.

#### Scenario: Five failed proofs

- GIVEN an open physical window
- WHEN five proofs fail
- THEN the window closes
- AND a further proof against that window authorizes nothing

#### Scenario: Window consumed
- GIVEN a successful claim or recovery
- WHEN the same window is presented again
- THEN it authorizes nothing further
