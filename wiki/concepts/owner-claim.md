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

1. **Device proof** -- Lightning Admin checks the node's signature on the challenge against the node id on the off-device card, before it derives anything from the secret. Developer SSH, discovery, and WPS are not ownership. hello.mesh only relays.
2. **Card HMAC** -- `k = HMAC-SHA256(salt, secret)`. The proof is `HMAC-SHA256(k, owner pk ‖ node id ‖ epoch ‖ nonce)`. The card is that secret (at least 80 bits of lowercase base32), not an NFC or Bluetooth device. The node does not display the secret, and the proof submission has no field for it.
3. **Button window** -- order is request, then press. A press arms a fresh 120-second window for the requests pending at press time. The window is not persisted. It closes on success, reboot, lost time, expiry, or five failed proofs. A sniffed proof does not replay: each attempt has a fresh nonce.

All three must match the same claim for ownership to be granted. A ceremony press arms the window. It does not toggle WAN SSH.

## Recovery

- Requires an enrolled recovery credential plus a new button window bound to the same request, node id, and current ownership epoch
- Success retires every prior recovery credential
- Does not elect a fork winner. A later successor names one parent

## Storage

The node stores `k` and the salt, not the card string. A successful first claim retires `k`. The setup button toggles WAN SSH when there is no verifier, and when the node is already owned and no recovery credential has verified (`ButtonEffect::ToggleWanSsh`). A verified recovery pending at press time arms the ceremony instead.

## Living Spec

`openspec/specs/household-owner-claim/spec.md`
