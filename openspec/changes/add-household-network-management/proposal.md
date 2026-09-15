# Add household network management

> **ACTIVE BUILD**

## Why

Lightning Mesh needs a consumer installation and ownership model for a co-living
house: existing upstream Wi-Fi, optional Ethernet, dual-band nodes, changing
residents, and shared local services. Current discovery, SSID apply, and public
identity do not provide secure fleet commissioning or topology policy.

## Scope

Implement the configuration model, capability-gated topology choices, ownership
and recovery, and shared web/desktop workflows through the `mjolnir-mesh-ai0`
dependency graph. Activation is authorization to build, not evidence that a
capability has shipped. Living specs remain unchanged until verified work folds.

Design: [technical model and hardware inputs](design.md).
Experience: [brief](../../../.design/household-network/DESIGN_BRIEF.md) and
[IA](../../../.design/household-network/INFORMATION_ARCHITECTURE.md).
Requirements: [draft delta](specs/household-network-management/spec.md).

## Relationship to existing work

- `mjolnir-mesh-bf7`: earlier fleet admin epic. Retain symmetric target-node
  authorization, fleet navigation, and reversible uplink configuration. Propose
  replacing hidden entry with the user-confirmed visible authenticated entry;
  raw owner-list mutation is addressed by the delegated authority proposal.
- `mjolnir-mesh-b6j.2`: reuse the proposed signed control envelope, installed signer,
  physical pairing, destination verification, and receipts. The newer
  [secure-context proposal](../../../docs/network-coordination/secure-context-and-control-plane.md)
  and [storage decisions](../../../docs/storage-node/decisions.md) were reviewed
  from fetched upstream before synchronization; they remain proposals.
- Existing settings remain under `/etc/config/mjolnir`; projection remains through
  `mjolnir-apply`. L3 client isolation across nodes remains the architectural baseline.

## User journey & surfaces

A purchaser enters Setup from Lightning Admin or the local web front desk, proves
possession, establishes a house owner, and selects an internet connection. Added
nodes inherit approved policy after claiming. The Overview explains actual client
connectivity and offers the next relevant action.

- Working: show active internet path, node health, Wi-Fi coverage, and local services.
- Empty: discover / pair a node; if none found, show physical connection guidance.
- Failed: preserve diagnosis and change receipts; provide reconnect or local recovery.
- Off: local-only is an intentional policy state; show “Enable internet” to admins.

Owners manage delegated access. Residents and guests use the public front desk;
an IdentiKey or `.mesh` name does not confer router administration.

## Confirmed decisions and remaining design work

Confirmed 2026-09-14: one initial owner, delegating to admins, members, and guests;
everyone is invited to hello.mesh and optional identity creation. The
[identity and welcome refinement](identity-and-welcome.md) records latest IdentiKey
evidence, Biscuit attenuation, FOKS-style keyspaces, and secure-context boundaries.

Also confirmed: house-owned network and visible authenticated **Manage network**
entry, with physical proof for claiming nodes. Secret-knock entry is superseded.

Also confirmed: private resident Wi-Fi by default, with optional isolated guest
Wi-Fi. Identity remains optional for internet access. This changes the target
household profile; it does not change the currently deployed open SSID.

Standalone phone-browser ownership still needs
a trusted-origin/bootstrap decision before it can be promised as supported.

## Activation

Future scope: [open backhaul and a paid right to pass](future-transit.md), potentially
settled through local-EVM token transfers. This is parked exploration, not an
initial requirement or a decision to open deployed backhaul.

Activated by Duke with `activate ai0` on 2026-09-14 (America/Los_Angeles).
This activation covers implementation of the existing ai0 dependency graph;
child changes inherit it when scaffolded. Dependency ordering, independent
architecture/instrument reviews, and human verification gates still apply.
Live-network changes require a separate explicit go-ahead; the working Symbio
uplink must not be disrupted by this software campaign. Paid transit remains parked.
Work state and implementation follow-ups are tracked in beads, not checkboxes here.
