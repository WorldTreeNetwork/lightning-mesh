---
title: Owner Claim
created: 2025-11-05
updated: 2026-11-05
type: concept
tags: [mesh-components, admin, owner-claim]
sources: [crates/mjolnir-mesh/src/owner_claim.rs, openspec/specs/household-owner-claim/spec.md]
state: [CURRENT]
confidence: high
---

# Owner Claim

The household owner claim is a ceremony that binds one owner to a node through device proof, a card HMAC, and a fresh 120-second local button window.

## Flow

1. **Device proof** -- the client verifies the node's meshd identity signature against the node id on the off-device card. Developer SSH, discovery, and WPS are not ownership.
2. **Card HMAC** -- the claim carries an HMAC of that request under the card secret. The card is that secret, not an NFC or Bluetooth device. The node does not display the secret.
3. **Button window** -- a fresh 120-second local window confirms the same request.

All three must match the same claim for ownership to be granted.

## Recovery

- Requires an enrolled recovery credential plus a new button window bound to the same request, node id, and current ownership epoch
- Success retires every prior recovery credential
- Does not elect a fork winner. A later successor names one parent

## Storage

The node stores `k` (the derived key), not the card string. The setup button toggles WAN SSH when there is no verifier, or when the node is already owned and no recovery credential has verified.

## Living Spec

`openspec/specs/household-owner-claim/spec.md`
