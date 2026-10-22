# add-house-hub-welcome

> **ACTIVE BUILD**

**Rigor:** architecture

Bead `mjolnir-mesh-ai0.2`, parent `mjolnir-mesh-ai0`
(`add-household-network-management`). Activation inherited from Duke's
`activate ai0` (2026-09-14 America/Los_Angeles) and this scoped run.
Implementation stays behind an accepting other-family review.

## Why

A visitor on the house network should find hello.mesh, look at local
information, and leave without creating an identity. Identity is an
invitation, not a ticket for internet or for router administration.
The captive sheet already explains offline state. It does not yet
promise a return path when that sheet never appears.

## What

- Capability `house-hub-welcome`: anonymous browsing, optional
  identity with consent, and a manual or QR return to the hub.
- Reuse the hello.mesh Services surface and the existing portal
  pass-through. Do not add a second front door.
- Dismissal creates no identity, membership, or capability.

## Impact

- Capabilities: ADDED `house-hub-welcome`.
- Living `captive-portal` stays as it is: a portal action does not
  grant or remove internet.
- ADRs: none.

## User journey & surfaces

A new visitor reaches hello.mesh from the offline portal or by opening
the advertised local URL or QR. Working: local information and services
are visible with no account. Empty: the hub still explains how to open
it when nothing is published yet. Failed: identity creation that cannot
keep a key shows a regular-browser path and does not pretend the portal
browser saved it. Off: “Just the internet, please” leaves no identity
behind, and local-only does not claim that dismissing the invitation
brought the internet back.

## Out of scope

- Mini-apps (`mjolnir-mesh-ncy`).
- Owner claim and recovery (`mjolnir-mesh-bf7.1`).
- Private resident Wi-Fi (`mjolnir-mesh-ai0.7`).
- Biscuit issuance and admin grants (`mjolnir-mesh-ai0.1`).
- Live fleet writes.
