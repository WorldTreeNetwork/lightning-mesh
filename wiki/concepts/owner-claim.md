---
title: Owner Claim
created: 2025-11-05
updated: 2025-11-05
type: concept
tags: [mesh-components, admin, owner-claim]
sources: [crates/mjolnir-mesh/src/owner_claim.rs, openspec/specs/household-owner-claim/spec.md]
state: [CURRENT]
confidence: high
---

# Owner Claim

The household owner claim is a ceremony that binds one owner to a node through device proof, a card HMAC, and a fresh 120-second local button window.

## Flow

1. **Device proof** -- physical access to the node (serial, USB, or local network)
2. **Card HMAC** -- a cryptographic card (NFC, Bluetooth, or similar) provides an HMAC challenge
3. **Button window** -- a 120-second window starting from the setup button press

All three must match the same claim for ownership to be granted.

## Recovery

- Requires an enrolled credential plus a new button window
- Advances the ownership epoch and the authority epoch
- Does not elect a fork winner

## Storage

The node stores `k` (the derived key), not the card string. The setup button toggles WAN SSH when there is no verifier, or when the node is already owned and no recovery credential has verified.

## Living Spec

`openspec/specs/household-owner-claim/spec.md`
