---
title: Deterministic Addressing from Identity
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, rayfish, identity, addressing]
sources: [public/rayfish/src/identity.rs, public/rayfish/src/membership.rs]
state: [FUTURE]
confidence: medium
---

# Deterministic Addressing from Identity

Adopt rayfish's model of deterministic IP addresses derived from cryptographic identity, instead of mesh's HLC-timestamped subnet claims.

## Concept

Rayfish derives stable IPs from the node's `SecretKey`:

- **IPv4:** `100.64.0.0/10` via FNV-1a hash of identity
- **IPv6:** `200::/7` via blake3 hash of identity

These never change across reboots. Mesh uses first-writer-wins `/24` claims from `10.42.0.0/16` with HLC timestamps, which can change when conflicts are resolved.

## Rayfish Model

```rust
// src/identity.rs - stable SecretKey persistence
pub fn load_or_create() -> Result<SecretKey> {
    if let Some(key) = load_existing()? {
        tracing::info!(id = %key.public().fmt_short(), "loaded identity");
        return Ok(key);
    }
    let key = SecretKey::generate();
    crate::config::write_file(&key_path()?, &key.to_bytes(), true)?;
    tracing::info!(id = %key.public().fmt_short(), "generated new identity");
    Ok(key)
}
```

Address derivation uses `membership::derive_ipv6()` (blake3) and `membership::derive_ipv4()` (FNV-1a). The `load_existing()` vs `load_or_create()` distinction is critical: `load_existing()` never auto-mints a key when the caller is about to restore one.

## Mesh Model

Mesh uses `10.42.0.0/16` with HLC-timestamped `/24` claims via `SubnetClaim` CRDT. Each node claims a `/24` and conflicts are resolved by HLC timestamp (first-writer-wins).

## Alignment Options

### Option A: Align on Rayfish's Ranges

Move mesh to `100.64.0.0/10` (IPv4) and `200::/7` (IPv6). Deterministic per-node addresses. No more HLC subnet conflicts.

**Pros:**
- One overlay range across both stacks. Cross-network traffic just works.
- No subnet claim CRDT needed. Addresses are derived, not negotiated.
- Operators can predict node IPs from their identity.

**Cons:**
- Breaking change for existing mesh deployments.
- `100.64.0.0/10` is shared with CGNAT. Rayfish handles this with overlay address filtering.
- `/7` IPv6 range is large but could be confusing for operators used to `/24` claims.

### Option B: Keep Mesh Ranges, Adopt Derivation Model

Keep `10.42.0.0/16` but derive addresses from identity instead of claiming. Each node gets a deterministic sub-range.

**Pros:**
- Less disruptive for existing deployments.
- Still eliminates HLC conflicts.

**Cons:**
- Ranges diverge between stacks.
- Still needs a claim/subnet mechanism for sub-range allocation.

### Option C: Hybrid -- Identity-Derived with HLC Fallback

Primary: deterministic from identity. Fallback: HLC claim if derivation conflicts (collision in hash space, extremely unlikely).

**Pros:**
- Best of both worlds.
- Minimal disruption.

**Cons:**
- More complex logic.

## Identity Persistence

Rayfish's `identity.rs` provides a clean pattern mesh can borrow:

- `load_existing()` -- returns `None` without minting (for restore scenarios)
- `load_or_create()` -- generates and persists only if no key exists
- `store_secret_key()` -- replaces identity (for restore)
- `store_device_cert()` / `load_device_cert()` -- user-signs-device cert model

Mesh's current identity model is simpler (single key, no device certs). Rayfish's device cert model (`DeviceCert` = user signs device key, persisted and verified on reload) could replace or augment mesh's owner-claim for multi-device setups.

## Device Cert Model

Rayfish's device cert allows a user key to sign a device key:

```rust
pub struct DeviceCert {
    pub user_identity: SecretKey,
    pub device_key: PublicKey,
    pub signature: Signature,
}
```

The cert is persisted, verified on reload, and can be revoked (delete + `unpair`). Mesh's owner-claim is a single-owner model. Rayfish's model supports multi-device with user-device hierarchy.

## Benefits

- Predictable, stable node addresses across reboots
- No HLC conflict resolution needed
- One overlay range simplifies cross-network routing
- Device cert model supports multi-operator setups

## Risks

- Breaking change if aligning ranges
- Hash collision in derivation space (extremely unlikely with blake3/FNV-1a)
- Rayfish's device cert model adds complexity to mesh's simpler owner-claim

## Effort

Medium. Range alignment is a config change + docs. Identity persistence pattern is ~50 lines. Device cert model is ~100 lines.
