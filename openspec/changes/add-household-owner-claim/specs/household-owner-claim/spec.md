## ADDED Requirements

### Requirement: Initial claim needs device proof, secret, and presence
The system SHALL bind one initial owner to an unclaimed node only when, for the
same claim request, the node's meshd identity key signs the request, the
one-time provisioning secret matches the hash stored on that node, and a fresh
local button window is open. Developer SSH, discovery, and the WPS protocol
SHALL NOT constitute ownership.

#### Scenario: Competing claim
- GIVEN an unclaimed node with one open physical window
- WHEN two different owner credentials race to claim it
- THEN at most one durable binding succeeds and a retry cannot overwrite it

#### Scenario: Secret without the button
- GIVEN the provisioning secret and a valid device signature
- WHEN no local button window is open
- THEN the claim is refused

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
hello.mesh HTTP code SHALL NOT receive the owner private key, the provisioning
secret after the hash comparison, or a reusable administrative bearer.

#### Scenario: Claim from the hub origin
- GIVEN a claim completed on the node
- WHEN hub response handlers run
- THEN they do not contain the owner private key or the raw provisioning secret

### Requirement: Physical window and admin grants use different clocks
The physical window SHALL default to 120 seconds and SHALL close on success,
reboot, or loss of trustworthy timing. Configuration-changing admin grants
SHALL use the household-authority profiles and class caps, not a separate
900 second default.

#### Scenario: Window consumed
- GIVEN a successful claim or recovery
- WHEN the same window is presented again
- THEN it authorizes nothing further
