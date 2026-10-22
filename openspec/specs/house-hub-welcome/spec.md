# house-hub-welcome

A visitor on the house network can open hello.mesh, read local
information, and leave without creating an identity. Identity is an
invitation, not a ticket for internet or for router administration.
Folded from `add-house-hub-welcome` (2026-10-22).

## Requirements

### Requirement: Visitors can use the hub without an identity

hello.mesh SHALL show local information and services to a client that
has presented no identity. Creating an identity SHALL be an invitation
with an explicit consent step before any public announcement of that
identity. Choosing to browse anonymously, dismissing the invitation,
or choosing “Just the internet, please” SHALL NOT create an identity,
a house membership, or an administrative capability.

#### Scenario: Visitor declines identity

- GIVEN a visitor sees the welcome invitation on hello.mesh
- WHEN they explore anonymously or choose “Just the internet, please”
- THEN no identity, membership, or capability is created
- AND local information remains available
- AND internet access stays whatever the network policy already is

### Requirement: The hub is reachable without the portal sheet

The system SHALL advertise a local URL or QR that opens hello.mesh
when the operating-system portal sheet is not shown and when upstream
internet is absent. That page SHALL offer local information and the
optional identity invitation. It SHALL NOT say that dismissing the
invitation restored internet.

#### Scenario: No internet and no portal sheet

- GIVEN local networking works, upstream internet is absent, and no portal sheet is shown
- WHEN a visitor opens the advertised local hub URL or QR
- THEN the hub offers local information and optional identity
- AND does not claim that dismissing the invitation restores internet

### Requirement: Portal identity creation does not leak key material

When identity storage in the portal browser is unverified, the flow
SHALL offer a regular-browser path and SHALL state that the portal
browser may not keep the key. The flow SHALL NOT place a private key
in a URL. The flow SHALL NOT ask the visitor for an existing owner seed.

#### Scenario: A portal browser cannot retain the identity

- GIVEN identity storage continuity to the regular browser is unverified
- WHEN the visitor chooses identity creation from the portal browser
- THEN the flow offers a regular-browser path and states the persistence limit
- AND the private key is not placed in a URL
- AND the flow does not request an existing owner seed
