---
title: Rayfish Invite and Admission Model
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, rayfish, invite, admission]
sources: [public/rayfish/src/invite.rs]
state: [FUTURE]
confidence: low
---

# Rayfish Invite and Admission Model

Adopt rayfish's invite and admission model for Lightning Mesh's node onboarding.

## Concept

Rayfish has a mature invite system (`src/invite.rs`, ~35K lines) for joining mesh networks. Mesh has a single-owner owner-claim ceremony. Rayfish's model supports multi-operator, network-level admission, and device hierarchies.

**Verified from invite.rs:** An invite code is `bs58(network_pubkey || coordinator || secret || checksum)`. The coordinator mints invites and is the *only* node that can verify and burn them. The ledger lives on disk at `~/.config/rayfish/invites/<network>.toml` with 0600 permissions. A `DeviceCert` (user signs device key) is persisted at `~/.config/rayfish/device_cert` as msgpack.

## Rayfish Invite Flow

1. **Coordinator** (network-key holder) creates a signed `GroupBlob` (pkarr-published network roster)
2. **Invitation** is a signed `ConnectMsg::InviteOffer` delivered via `INVITE_ALPN`
3. **Welcome** includes the `GroupBlob`, network key grant, and `direct_key` (coordinator's direct connection key)
4. **Member** converges to the live roster in ~1s via `ControlMsg::SignedRecord`
5. **Nullifiers** for revocation -- revoking a device adds its nullifier to the next `GroupBlob`

## Mesh Current Model

Mesh uses `openspec/specs/household-owner-claim/spec.md`: device proof + card HMAC + button window. Single owner per node. No multi-device model.

## Key Differences

| Aspect | Mesh | Rayfish |
|---|---|---|
| Owner model | Single owner per node | Coordinator + multi-device |
| Admission | Device proof + card HMAC | Signed invite + GroupBlob |
| Key hierarchy | Single key | User key signs device key |
| Revocation | Full ownership transfer | Nullifier in GroupBlob |
| Multi-device | No | Yes (pair/unpair) |
| Network scope | Household | Named networks |

## Integration Options

### Option A: Replace Owner-Claim with Invite

Replace mesh's owner-claim with rayfish's invite model.

**Pros:**
- Multi-operator support
- Device hierarchies
- Mature, battle-tested model
- Nullifier-based revocation

**Cons:**
- 40K lines of code
- Deeply coupled to rayfish's daemon architecture
- Breaking change for existing deployments

### Option B: Borrow Selective Patterns

Adopt specific rayfish patterns without copying the full system:

- **Signed invites** -- coordinator signs an invitation blob that a node can present
- **Nullifiers** -- simple revocation token
- **Device certs** -- user signs device key (100 lines, not 40K)

**Pros:**
- Minimal code change
- Adds multi-device support
- Keeps mesh's simpler owner-claim as fallback

**Cons:**
- Partial integration, not full feature parity

### Option C: Keep Owner-Claim, Add Device Certs

Keep mesh's owner-claim as the primary model. Add rayfish's device cert pattern for multi-device setups.

**Pros:**
- Minimal disruption
- Adds device hierarchies without changing owner model
- Device cert pattern is only ~100 lines

**Cons:**
- Two admission models to maintain
- Inconsistent UX

## Benefits

- Multi-operator mesh networks
- Device hierarchies (user signs device)
- Nullifier-based revocation
- Network-scoped admission

## Risks

- 40K lines is too large to copy wholesale
- Owner-claim is field-validated on 5 routers; changing it risks breaking deployments
- Two admission models add operational complexity

## Effort

High if copying (40K lines). Low-Medium if borrowing patterns (~200 lines for device certs + signed invites).
