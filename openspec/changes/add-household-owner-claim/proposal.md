# add-household-owner-claim

> **ACTIVE BUILD**

**Rigor:** instrument

## Why

The person who bought the node has to become its owner without a developer SSH
key, and a neighbor who can press a button or see a setup screen must not be
able to take that role. Recovery has to require an enrolled credential and
physical presence together.

## What

- Bind the first owner only when device proof, a one-time provisioning secret,
  and a fresh physical window all match the same claim.
- Recover or transfer ownership only with an enrolled recovery credential plus
  a new physical window, then retire the old recovery credentials.
- Leave forked recoveries unverified until one successor exists.
- Keep owner private keys out of hello.mesh.

## Impact

- Capabilities: ADDED household-owner-claim.
- ADRs: none. The ceremony pins are in design.md.
- Work node: mjolnir-mesh-bf7.1. Activation inherited from Duke's `activate ai0`.
  No live network writes.
- Administrative grant lifetimes stay the household-authority profiles. The
  older 900 second / 3600 second note is not this change's clock.
- Chain verification is `add-control-record-integrity`. This change writes heads.

## User journey & surfaces

A purchaser, on their own device, checks the node id printed on the card,
enters the card secret there, and presses the node's setup button within the
window. The node never shows the secret. The node binds that owner. A second person racing the same window does not overwrite the
binding. Later, the owner recovers from Lightning Admin only after enrolling a
recovery credential and pressing the button again. The button alone, or the
credential from another network, leaves the current owner in place. hello.mesh
never asks for or receives the owner private key.

## Out of scope

- WPS protocol and the existing WPS WAN-SSH window (presence only; not ownership).
- Biscuit issuance internals (`mjolnir-mesh-ai0.1`) beyond consuming the authority epoch.
- Private resident Wi-Fi, paid transit, and live button qualification on the fleet.
- Folding the household umbrella.
