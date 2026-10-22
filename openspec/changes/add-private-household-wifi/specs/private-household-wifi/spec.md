# private-household-wifi

## ADDED Requirements

### Requirement: The house has one client SSID

The house SHALL use one client SSID. It SHALL NOT add a second SSID
for visitors, for a QoS class, or for a guest prefix. Recording this
intention SHALL NOT change the deployed SSID. Only a later
owner-committed plan SHALL encrypt or rename that SSID, and that plan
is not this change.

#### Scenario: Decision recorded, radios untouched

- GIVEN the deployed client SSID is open
- WHEN the house intention is recorded with `commit` false
- THEN the plan's radio effect is unchanged
- AND no second SSID is added

### Requirement: Three classes share that SSID

The plan SHALL classify each client flow as a published service, a
member, or unidentified. A published `.mesh` name SHALL be reachable
from every client on the SSID. Clients SHALL NOT gain a route to every
house address. The management plane SHALL stay closed to client
traffic, including overlay `10.254.0.0/16`, WAN SSH, and the recovery
address. A member SHALL be a client whose IdentiKey guild or group
membership the node has verified. When the link is busy, member flows
SHALL be scheduled ahead of unidentified flows. The plan SHALL NOT
impose a rate cap. An unidentified client SHALL keep internet and
published names at best effort. Creating an identity SHALL NOT be
required for that access. SSID association SHALL NOT count as
membership, and a directory role label SHALL NOT be the priority mark.

#### Scenario: Member and unidentified client share a busy link

- GIVEN one client SSID and a busy link
- WHEN a verified member and an unidentified client both send
- THEN the member flow is scheduled ahead
- AND neither flow is rate-capped

#### Scenario: Unidentified client opens a published name

- GIVEN a box on a node has published a `.mesh` name
- WHEN an unidentified client on the SSID opens that name
- THEN the plan allows it
- AND the client is not given a route to every house address
- AND the management plane stays closed

#### Scenario: Association is not membership

- GIVEN a client has associated to the SSID and has no verified membership
- WHEN the scheduler classifies the flow
- THEN the flow is unidentified

### Requirement: The builder does not submit to the live adapter

The class plan SHALL be a pure function. It SHALL NOT call
`OpenWrtAdapter`. It SHALL NOT bridge client L2 segments across nodes.
It SHALL NOT invent a priority mark while `mjolnir-mesh-ai0.1` has not
produced a verifier the node can check.

#### Scenario: Builder is invoked in tests

- GIVEN a fake adapter
- WHEN the class plan is built
- THEN `OpenWrtAdapter` is not constructed or called
