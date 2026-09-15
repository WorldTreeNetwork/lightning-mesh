# household-network-management

> Proposed requirements. PENDING; no claim of current implementation.

## ADDED Requirements

### Requirement: Visible authenticated administration entry

The public hub SHALL provide a visible “Manage network” entry. Privileged actions
SHALL require authentication and scoped destination authorization. Claiming a new
node SHALL require physical proof; discovering the entry SHALL NOT grant access.

#### Scenario: Visitor opens Manage network

- GIVEN a visitor without an administrative grant
- WHEN they follow the visible Manage network link
- THEN they can reach the authentication/access guidance
- AND cannot perform privileged operations or claim a node without required proof

### Requirement: One initial owner delegates scoped capabilities

The product SHALL begin with one owner and offer Admin, Member, and Guest presets
as scoped grants. Agency SHALL use the IdentiKey Biscuit format with a defined
mesh profile, destination verification, and no authority inferred from membership.
A second owner SHALL NOT be required for setup. Recovery SHALL be explicit.

#### Scenario: Admin exceeds a delegated node scope

- GIVEN an admin grant permits Wi-Fi changes on Hall only
- WHEN the holder requests an Office change or ownership transfer
- THEN the destination refuses even if the holder is a house keyspace member

#### Scenario: Holder-bound delegation names another person

- GIVEN a grant requires Alice's verified holder identity
- WHEN Alice attempts to delegate to Bob by only appending a Bob-holder check
- THEN the system does not treat Alice's check as removed
- AND a new recipient requires authorized scoped issuance within the parent ceiling

### Requirement: Connecting visitors can discover the hub without signing up

The experience SHALL invite visitors to hello.mesh and optional identity creation,
with anonymous exploration, consent before public announcement, a dismissal path,
and manual/QR discovery independent of automatic portal presentation. Browser-only
passkeys SHALL NOT be a prerequisite for this welcome experience.

#### Scenario: Visitor declines identity

- GIVEN a visitor sees the welcome invitation
- WHEN they explore anonymously or choose “Just the internet, please”
- THEN no identity, membership, or capability is created
- AND hello.mesh remains discoverable and internet access follows network policy

#### Scenario: No internet and no portal sheet

- GIVEN local networking works but upstream internet and automatic portal presentation do not
- WHEN a visitor opens the advertised local hub URL
- THEN the hub offers local information/services and optional identity
- AND does not claim that dismissing the invitation restores internet

#### Scenario: A portal browser cannot retain the identity

- GIVEN identity storage continuity to the regular browser is unverified
- WHEN the visitor chooses identity creation
- THEN the flow offers a regular-browser path with clear persistence limits
- AND never transports the private key through a URL or requests an existing owner seed

### Requirement: Separate intent, capability, and observed state

The system SHALL represent internet connections, node connections, client Wi-Fi,
and administrative ownership separately. It SHALL expose house policy, node
overrides, actual state, and observation age without equating discovery to trust.

#### Scenario: A wired node has two available client radios

- GIVEN a claimed node with a verified Ethernet node path and qualified dual-AP support
- WHEN its mesh radio is not required by dependants or fallback policy
- THEN the system offers both client bands with a review of the fallback tradeoff
- AND does not claim that this profile is supported solely from hardware marketing

#### Scenario: A nearby node is unowned by this house

- GIVEN a nearby node with a different house grant
- WHEN discovery finds it
- THEN the system does not adopt it, send secrets, or authorize configuration

### Requirement: Capability-gated Wi-Fi internet

The system SHALL offer upstream Wi-Fi only on eligible nodes with a surviving
downstream path. Candidate ranking SHALL distinguish scan estimates from dated
end-to-end tests and disclose client/relay service lost by the assignment.

#### Scenario: Both radios would be consumed

- GIVEN a dual-radio node needing one station radio and one mesh radio
- WHEN the admin reviews it as an upstream receiver
- THEN the preview states that this node has no client radio in that profile
- AND checks that peer mesh bands/channels are compatible before applying

#### Scenario: Signal winner would isolate a branch

- GIVEN the strongest upstream signal is on the sole relay for a branch
- WHEN its proposed role would remove that branch's only path
- THEN it is excluded unless a validated replacement path is included in the plan

### Requirement: Internet success requires relevant evidence

The system SHALL distinguish association, addressing, DNS, verified internet,
sign-in required, intentional local-only, and unknown states. Commissioning SHALL
verify traffic through the chosen uplink and a downstream client path.

#### Scenario: DHCP succeeds but upstream has no internet

- GIVEN a candidate obtains a lease but internet verification fails
- WHEN setup reports the result
- THEN it does not report “Internet connected” and identifies the failed stage

#### Scenario: Another gateway could mask candidate failure

- GIVEN an existing working gateway and a new candidate
- WHEN testing the candidate
- THEN probes are constrained to that candidate rather than the existing default route

### Requirement: Wired topology preserves trust and routing boundaries

The system SHALL validate port roles and loop safety before accepting Ethernet
as a node path. It SHALL preserve routed per-node client networks unless a
separately approved architectural change defines another mode.

#### Scenario: Ambiguous switch attachment

- GIVEN Ethernet carrier without proof whether the segment is WAN, node link, or client LAN
- WHEN the cable is detected
- THEN the UI requests intended role before a disruptive reassignment
- AND does not merge client bridges or enable competing DHCP servers

### Requirement: Changes have durable outcomes and recovery

All disruptive configuration SHALL use revision-checked plans through
`mjolnir-apply`, with persisted recovery, deadlines, target authorization, and
per-node receipts. Fleet plans SHALL account for dependencies and partitions.

#### Scenario: Browser closes during apply

- GIVEN targets have prepared a disruptive plan
- WHEN the browser disconnects or closes
- THEN target recovery continues independently
- AND a later session can retrieve committed, restored, partial, or unknown outcomes

#### Scenario: Concurrent editors

- GIVEN a plan was drafted against an old configuration revision
- WHEN another administrator commits a change first
- THEN the stale plan is rejected for review rather than overwriting it

#### Scenario: Mesh channel migration has an unprepared node

- GIVEN an affected node cannot prepare recovery and coordinated activation
- WHEN a channel migration is requested
- THEN the migration is declined before isolating the node and offers a maintenance path

### Requirement: Authorization is verified at the destination

The destination SHALL verify scoped signed operations, authority revision,
freshness, replay protection, and target binding. Discovery, public identity,
SSID knowledge, and unsigned replicated owner lists SHALL NOT confer authority.

#### Scenario: Public identity attempts router control

- GIVEN a resident has a valid IdentiKey or name claim but no admin grant
- WHEN they request a radio change
- THEN the target rejects the operation regardless of entry-node behavior

#### Scenario: A proxy alters or repeats a command

- GIVEN a signed request binding target, action, arguments, and request ID
- WHEN an intermediary changes it or replays it outside permitted idempotent handling
- THEN the target rejects it without executing a second mutation

### Requirement: Claiming and recovery do not depend on installer SSH keys

Shipped nodes SHALL use unique device bootstrap material and physical proof for
claiming. They SHALL contain no developer owner or SSH backdoor. Owner credentials
SHALL use trusted custody, with a verified recovery/transfer path.

#### Scenario: Nearby attacker races initial setup

- GIVEN the purchaser opens a physical pairing window
- WHEN an observer without per-device proof submits a claim
- THEN the claim is rejected even if it arrives first

#### Scenario: Last owner would be removed

- GIVEN no verified replacement or recovery authority
- WHEN the last owner is removed
- THEN removal is blocked until continuity is established

#### Scenario: Revocation reaches a partitioned fleet

- GIVEN one node is disconnected during credential revocation
- WHEN the owner views the result
- THEN enforcement on that node is marked pending
- AND expired authority cannot authorize new privileged changes there
- AND reconnect reconciles revocations before accepting fresh grants

### Requirement: Web and desktop share administrative semantics

Both surfaces SHALL use the same plan, role, validation, and receipt contracts.
Platform custody and discovery limitations SHALL be explicit. Plain HTTP public
identity storage SHALL NOT be treated as owner credential custody.

#### Scenario: Browser lacks trusted owner signing

- GIVEN the local web interface is reached over an untrusted HTTP origin
- WHEN owner setup or a privileged mutation is requested
- THEN it offers the supported trusted-signer path
- AND does not request or import an owner private seed into that page

### Requirement: Network access and identity have distinct scopes

Internet access SHALL follow network policy without mandatory community identity.
Guest isolation SHALL cover cross-node, wired, wireless, IPv4, and enabled IPv6
paths before the UI labels a profile isolated.

#### Scenario: Guest joins another node

- GIVEN an isolated guest profile and a private resident service on a different node
- WHEN the guest tries to reach that service over any enabled address family
- THEN policy denies access unless an explicit service exception exists

#### Scenario: Upstream disappears

- GIVEN local services and node links remain healthy
- WHEN internet verification fails
- THEN local management/services remain available and the UI reports the upstream failure separately
