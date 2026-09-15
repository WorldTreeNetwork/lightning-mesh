# Design Brief: A house network people can install and own

> Draft for discussion, 2026-09-14. Change: `add-household-network-management`.
> Planning deliverable: `mjolnir-mesh-08x`. No router configuration changed.
> Recommended defaults below are proposals, not accepted decisions or shipped behavior.

## Problem

Someone buys several Lightning Mesh nodes, connects one to their router, and
places the others around a house. They expect working internet and local services.
They should not have to understand radio allocation, Babel, SSH keys, or which
node they happened to connect to. When the network cannot satisfy their needs,
they need an explanation and a useful next action.

The initial setting is an SF co-living Victorian with uncertain cable routes,
existing upstream Wi-Fi, dual-band nodes, and an available Ethernet switch.
The current installation has no internet egress according to the operator;
this session reviewed source and historical measurements, not live hardware.

## Solution

Set up a **house network** once. Add nodes through proof of possession. The
network selects validated connections within the owner's policy and explains
the result in terms of coverage, internet, and local performance. A phone can
guide placement; the desktop supports commissioning and detailed fleet work.
Both show the same plans, permissions, and change receipts.

Separate three questions throughout the experience:

| Question | Product label | Examples |
|---|---|---|
| Where does internet come from? | Internet connection | Existing router, upstream Wi-Fi, later cellular |
| How do nodes reach each other? | Node connections | Ethernet, wireless mesh, mixed |
| What do people join? | Wi-Fi networks | Residents, Guests, optionally Devices |

“Backhaul” may appear in technical details. “Wireless bridge” is not the default
label: the proposed ordinary Wi-Fi uplink is routed, usually with NAT, not a
promise of a transparent Ethernet bridge.

## Experience principles

1. **Explain the outcome.** Show “5 GHz available in these rooms” and the route
   to internet before showing interface names or channel numbers.
2. **Automate within a promise.** Prefer working wired paths; preserve declared
   coverage and recovery requirements. Propose changes that break those promises.
3. **Make access durable.** A house outlives its installer and individual laptops.
   Ownership, delegation, revocation, and recovery are first-class setup jobs.

## People and projected usage

These are hypotheses for field validation, not interview findings.

| Person / situation | Job and frequency | Entry and successful outcome |
|---|---|---|
| Purchaser / installer | Once: bring a kit online | Setup; a client reaches internet and all claimed nodes appear |
| Resident | Daily: internet, calls, local apps | Public front desk; no admin ceremony needed |
| House admin | Occasionally: diagnose a complaint | Overview; distinguish room Wi-Fi, node path, and upstream failure |
| Installer walking the house | During installation / changes | Phone node view; identify node, test a room, compare placement |
| AI developer | Often: reach a workstation, inference host, NAS | Services; service access independent of router ownership |
| Guest / event host | Per visit / event | Guest connection or invitation; no resident or admin access by accident |
| Departing admin / replacement owner | Rare, high consequence | People & access; transfer authority and prove recovery |

Admins should spend most console time on Overview and its actionable exceptions.
Residents should rarely need the console at all. A topology map supplements a
plain explanation; it is not the landing-page prerequisite.

## Confirmed direction and recommended defaults

- Confirmed 2026-09-14: one initial owner delegates to admins, members, and guests.
  A second owner is not required. Recovery setup is separate from delegation.
- Confirmed: invite every connecting person to discover hello.mesh and optionally
  create an identity. Browsing and internet do not require identity creation.
- Proposed implementation: Biscuit agency capabilities and the existing IdentiKey
  shared-keyspace model, with its runtime gaps explicitly tracked. See the
  [identity and welcome refinement](../../openspec/changes/add-household-network-management/identity-and-welcome.md).
- Confirmed: visible **Manage network** entry protected by authentication and
  authorization, with physical proof when claiming a node. This supersedes the
  secret-knock entry in the [older IA](../fleet-admin/INFORMATION_ARCHITECTURE.md).
- Private resident Wi-Fi with optional isolated guests; optional identity for
  internet. Current open SSID remains current behavior until separately changed.
- A wired-first **Balanced** policy. Reassigning a radio that removes coverage
  requires an impact preview and confirmation unless that exact tradeoff was
  authorized in the selected policy.
- Local management and local services remain usable without internet.

## Aesthetic direction

- Philosophy: a calm instrument panel with familiar language and inspectable detail.
- Tone: confident about measured facts; explicit about stale or unknown data.
- Reference: extend Lightning Admin's compact dark/mint controls and hello.mesh's
  cards, status labels, and topology. No external visual reference was requested.
- Avoid unexplained base58 addresses as primary node names, alarming red for
  intentionally local-only operation, and a screen full of equally prominent CTAs.

## Existing patterns

- Desktop: Svelte 5 + SvelteKit static SPA inside Tauri 2. `admin/src/app.css`
  defines dark green panels, mint accent, amber warning, and red failure tokens;
  font stack starts with IBM Plex Mono / JetBrains Mono, 13px root.
- Web: SvelteKit, Tailwind, shadcn-svelte primitives; semantic light/dark variables
  in `hello-mesh-web/src/routes/layout.css`. Existing main layout uses `gap-6 p-6`.
- Existing web vocabulary: HeroStrip, IdentityManager, RoutersPanel, RadioGraph,
  PeoplePanel, ServicesPanel, StampCoords, cards, popovers, badges, buttons.
- Desktop discovery already supports scan, interface/scope filters, copying addresses,
  and fleet network-name apply reports. Preserve those under technical details.
- No shared cross-app token package or Storybook was found in the inspected trees.
  Share semantic status names and behavior first; visual unification is follow-on work.

## Component inventory

| Component | Status | Change |
|---|---|---|
| Discovery store / address row | Modify | Verified node identity, model, room label, claim state |
| RadioGraph / RoutersPanel | Modify | Wired links, direction to internet, freshness, selected route |
| IdentityManager / PeoplePanel | Modify | Keep community identity separate from admin authority |
| ServicesPanel | Modify | Visibility/access labels; local availability during upstream outage |
| Network-name form | Modify | Wi-Fi profile scope, preview, per-node outcomes |
| Setup guide and physical pairing | New | First owner, add node, recovery preparation |
| Internet connection chooser | New | Upstream scan, candidate ranking, credentials, tests |
| Radio allocation summary | New | Per node: 2.4 GHz, 5 GHz, Ethernet purpose and consequences |
| Change preview and receipt | New | Impact, target versions, progress, restored / unknown states |
| Roles, invitations, recovery | New | Permission scope, expiry, revocation, ownership transfer |
| Room test | Extend | Use coverage work; record actual walk, do not invent a floorplan heatmap |

## Key interactions

**Connect internet:** start from “Local network working; internet unavailable.”
Choose cable or Wi-Fi. For Wi-Fi, eligible nodes scan sequentially, then the UI
shows a recommendation with its downstream path and coverage cost. Enter the
password in the trusted setup surface. Review, test, and apply. Failure says
whether association, addressing, DNS, or internet verification failed.

**Add Ethernet:** show the detected port and peer, negotiate a trusted node link,
verify reachability, then propose releasing an unneeded mesh radio. First check
whether other nodes depend on that node as a wireless relay. A cable does not
by itself authorize joining an existing trust domain.

**Move a node:** identify it, label its room, measure the current location, move,
and compare. A healthy mesh link does not prove good client coverage. Report
both and record where/when the test happened.

**Apply a fleet change:** preview lost bands, affected clients/nodes, expected
interruption, and fallback. Progress remains attached to a transaction ID even
if the browser closes. On reconnect, fetch the result; do not infer success.

**Invite another admin:** choose a role and scope, issue an expiring invitation,
bind it to the recipient's credential on acceptance, and show pending versus
confirmed enforcement across nodes.

## Responsive behavior

- Phone: single-column tasks, Overview / Nodes / More navigation; forms use full
  pages, node facts appear as cards, and topology always has a text equivalent.
- Desktop: persistent five-section rail, list/detail layout, bulk selection and
  keyboard navigation. Bulk changes still produce per-node receipts.
- Never require hover to discover essential facts. Keep the active task and its
  reconnect instructions visible during interruption.

## Accessibility requirements

Target WCAG AA: 4.5:1 normal text and 3:1 large text / meaningful UI graphics;
verify final colors in implementation. Visible focus, semantic headings,
labelled fields, keyboard-complete flows, status text alongside color, reduced
motion, 200% zoom, and 44px touch targets where practical. Announce major async
state changes without constantly reading telemetry. Keep entered non-secret
values after errors; focus the error summary and then the offending field.

## Success criteria and UX validation

Pilot goals, not measured results: a new purchaser gets a three-node kit online
without shell commands; an admin can explain a proposed radio tradeoff before
applying; a resident can distinguish upstream outage from local connectivity;
the owner uses verified recovery after the installer's laptop is unavailable.
Visitors can find hello.mesh again after dismissing the welcome invitation.

Run observed tasks with a purchaser, resident, and technical admin: cable setup,
Wi-Fi-only setup, bad password, weak room, cable removal, invitation, and owner
recovery. Record completion, help requests, terminology errors, and mistaken
success interpretations. Test web and desktop against identical fixture states.

## Out of scope

This deliverable does not implement or deploy firmware, change the live SSID,
choose a new hardware SKU, promise seamless roaming, implement collective voting,
or provide cloud management. Hardware qualification and browser trust are explicit
design inputs in the linked technical design, not solved by drawing screens.

## Reading order

1. [Configuration and hardware design](../../openspec/changes/add-household-network-management/design.md)
2. [Information architecture and workflows](INFORMATION_ARCHITECTURE.md)
3. [Proposed requirements and scenarios](../../openspec/changes/add-household-network-management/specs/household-network-management/spec.md)
