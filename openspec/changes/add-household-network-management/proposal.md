# Add household network management

> **PENDING**

## Why

Lightning Mesh needs a consumer installation and ownership model for a co-living
house: existing upstream Wi-Fi, optional Ethernet, dual-band nodes, changing
residents, and shared local services. Current discovery, SSID apply, and public
identity do not provide secure fleet commissioning or topology policy.

## Scope

Propose a configuration model, capability-gated topology choices, ownership and
recovery, and shared web/desktop workflows. The planning artifact is complete
independently of implementation. No living spec is changed or capability shipped.

Design: [technical model and hardware inputs](design.md).
Experience: [brief](../../../.design/household-network/DESIGN_BRIEF.md) and
[IA](../../../.design/household-network/INFORMATION_ARCHITECTURE.md).
Requirements: [draft delta](specs/household-network-management/spec.md).

## Relationship to existing work

- `mjolnir-mesh-bf7`: earlier fleet admin epic. Retain symmetric target-node
  authorization, fleet navigation, and reversible uplink configuration. Propose
  revisiting hidden entry and raw owner-list mutation; do not silently supersede it.
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

## Decisions awaiting review

House versus individual ownership; visible admin entry versus previous secret
knock; private resident Wi-Fi versus current open default. Recommended choices
are explicitly marked in the brief. Standalone phone-browser ownership also needs
a trusted-origin/bootstrap decision before it can be promised as supported.

## Activation

This is a planning proposal. Implementation remains pending review and activation.
Work state and implementation follow-ups are tracked in beads, not checkboxes here.
