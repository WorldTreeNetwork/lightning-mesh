# household-network-management

> Reconciled 2026-11-03. Folded slices live in `openspec/specs/`. The
> requirements below are still proposed. They are not a claim that the
> folded slices are unimplemented.

## Landed elsewhere

- One client SSID, member and unidentified flows, published names, management plane closed, builder does not touch the live radio: `private-household-wifi`. Live radio apply remains `mjolnir-mesh-lpv`.
- hello.mesh without forced identity: `house-hub-welcome`.
- Owner ceremony and two-factor recovery: `household-owner-claim`.
- Durable plans, receipts, and recovery: `network-change-transactions`.
- Port roles and wired-core separation: `node-connection-profiles`.
- Station uplink, lease is not internet, restore on failure: `wifi-internet-uplink`.
- Consumers verify owner-signed admin records: `admin-record-integrity`.

The accepted trust contract is bead `mjolnir-mesh-ai0.3`. It does not ship the grant engine.

## ADDED Requirements

### Requirement: Visible authenticated administration entry

The public hub SHALL provide a visible “Manage network” entry. Privileged actions
SHALL require authentication and scoped destination authorization. Claiming a new
node SHALL require physical proof; discovering the entry SHALL NOT grant access.

Owed by `mjolnir-mesh-bf7.7` and `mjolnir-mesh-b6j.2`. Secret-knock entry is superseded.

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

The ceremony is `household-owner-claim`. Issuance and destination checks remain
`mjolnir-mesh-ai0.1`. Signed transport remains `mjolnir-mesh-b6j.2`.

#### Scenario: Admin exceeds a delegated node scope

- GIVEN an admin grant permits Wi-Fi changes on Hall only
- WHEN the holder requests an Office change or ownership transfer
- THEN the destination refuses even if the holder is a house keyspace member

#### Scenario: Holder-bound delegation names another person

- GIVEN a grant requires Alice's verified holder identity
- WHEN Alice attempts to delegate to Bob by only appending a Bob-holder check
- THEN the system does not treat Alice's check as removed
- AND a new recipient requires authorized scoped issuance within the parent ceiling

### Requirement: Separate intent, capability, and observed state

The system SHALL represent internet connections, node connections, client Wi-Fi,
and administrative ownership separately. It SHALL expose house policy, node
overrides, actual state, and observation age without equating discovery to trust.

Port roles are `node-connection-profiles`. This requirement is the remaining
observation and adoption behavior. Qualification is `mjolnir-mesh-lpv`.

#### Scenario: A wired node has two available client radios

- GIVEN a claimed node with a verified Ethernet node path and qualified dual-AP support
- WHEN its mesh radio is not required by dependants or fallback policy
- THEN the system offers both client bands with a review of the fallback tradeoff
- AND does not claim that this profile is supported solely from hardware marketing

#### Scenario: A nearby node is unowned by this house

- GIVEN a nearby node with a different house grant
- WHEN discovery finds it
- THEN the system does not adopt it, send secrets, or authorize configuration

### Requirement: Authorization is verified at the destination

The destination SHALL verify scoped signed operations, authority revision,
freshness, replay protection, and target binding. Discovery, public identity,
SSID knowledge, and unsigned replicated owner lists SHALL NOT confer authority.

Record verification is `admin-record-integrity`. The signed operation and receipt
path remains `mjolnir-mesh-b6j.2`.

#### Scenario: Public identity attempts router control

- GIVEN a resident has a valid IdentiKey or name claim but no admin grant
- WHEN they request a radio change
- THEN the target rejects the operation regardless of entry-node behavior

#### Scenario: A proxy alters or repeats a command

- GIVEN a signed request binding target, action, arguments, and request ID
- WHEN an intermediary changes it or replays it outside permitted idempotent handling
- THEN the target rejects it without executing a second mutation

### Requirement: Revocation stays honest across a partition

Removing the last owner SHALL wait for a verified replacement or recovery
authority. A node that missed a revocation SHALL show enforcement as pending
and SHALL NOT accept a fresh grant from expired authority. Reconnect SHALL
reconcile revocations before accepting fresh grants.

This is not the owner ceremony. It remains with `mjolnir-mesh-ai0.1`.

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

Owed by `mjolnir-mesh-bf7.7`.

#### Scenario: Browser lacks trusted owner signing

- GIVEN the local web interface is reached over an untrusted HTTP origin
- WHEN owner setup or a privileged mutation is requested
- THEN it offers the supported trusted-signer path
- AND does not request or import an owner private seed into that page
