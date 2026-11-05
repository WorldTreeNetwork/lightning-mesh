---
title: Rayfish Identity and Device Cert Model
created: 2026-11-05
updated: 2026-11-05
type: interop
tags: [interop, rayfish, identity, device-cert]
sources: [public/rayfish/src/identity.rs, public/rayfish/src/membership.rs]
state: [CURRENT]
confidence: high
---

# Rayfish Identity and Device Cert Model

Grounded in verified source code. Rayfish uses a single Ed25519 key for identity, with an optional device cert model for multi-device setups.

## Key Model (identity.rs)

### Single Ed25519 Key

**Verified from identity.rs:** Rayfish uses ONE `iroh::SecretKey` (Ed25519, 32 bytes) — NOT a dual X25519+Ed25519 key like Reticulum. The key file is exactly 32 raw bytes at `~/.config/rayfish/secret_key`.

```rust
// File format: 32 raw bytes
pub fn load_existing() -> Result<Option<SecretKey>> {
    let bytes: [u8; 32] = std::fs::read(&path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("corrupt secret key file"))?;
    Ok(Some(SecretKey::from_bytes(&bytes)))
}
```

### Key Functions

| Function | Behavior |
|---|---|
| `load_existing()` | Returns `None` without minting (for restore scenarios) |
| `load_or_create()` | Generates and persists only if no key exists |
| `store_secret_key()` | Replaces identity (for restore) |
| `key_path()` | Returns `~/.config/rayfish/secret_key` |

### Mesh vs Rayfish Identity

| Aspect | Mesh | Rayfish |
|---|---|---|
| Key type | iroh::SecretKey (Ed25519) | iroh::SecretKey (Ed25519) |
| Key size | 32 bytes | 32 bytes |
| Separate X25519 | No | No (same as mesh) |
| Device certs | No | Yes |
| Storage | `~/.config/mjolnir/identikey.json` | `~/.config/rayfish/secret_key` |
| Multi-device | No (single owner) | Yes (user signs device) |

## Device Cert Model (identity.rs)

### DeviceCert Struct

**Verified from identity.rs:**
```rust
pub struct DeviceCert {
    pub user_identity: SecretKey,  // User's iroh::SecretKey
    pub device_key: PublicKey,      // Device's iroh::SecretKey.public()
    pub signature: Signature,       // User signed device_key
}
```

### Device Cert Lifecycle

1. **Create**: User signs device key → `DeviceCert::create(&user, &device, 0)`
2. **Store**: `store_device_cert(&cert)` → `~/.config/rayfish/device_cert` (msgpack)
3. **Verify**: `load_device_cert()` → `cert.verify()` on reload
4. **Delete**: `delete_device_cert()` → idempotent

### Mesh Application

Rayfish's device cert model can replace or augment mesh's owner-claim for multi-device setups:
- User key = mesh node's identity
- Device key = sub-device (phone, laptop) signed by user
- Revocation = delete cert + `unpair`

## Address Derivation (membership.rs)

### IPv6 — `derive_ipv6()`

**Verified from membership.rs:**
```rust
pub fn derive_ipv6(identity: &EndpointId) -> Ipv6Addr {
    let mut hex = [0u8; 64];  // hex on stack, zero allocation
    hex::encode_to_slice(identity.as_bytes(), &mut hex);
    let hash = blake3::hash(&hex);
    let octets: [u8; 16] = [0x02, hash.as_bytes()[0..15]...];
    Ipv6Addr::from(octets)  // 200::/7 range
}
```

- **120-bit address space** (2^120) — collisions "practically impossible"
- **Zero allocation** — hex encoding done on stack
- **Never carried on the wire** — every node computes every peer's address

### IPv4 — FNV-1a

- Range: `100.64.0.0/10` (CGNAT)
- Overlay is **IPv6-only** — IPv4 only for exit node egress

## Member Roster (membership.rs)

### Member Struct

**Verified from membership.rs:**
```rust
pub struct Member {
    pub identity: EndpointId,        // Device's iroh::EndpointId
    pub is_coordinator: bool,
    pub hostname: Option<String>,
    pub user_identity: Option<EndpointId>,  // Paired user key
    pub device_cert: Option<DeviceCert>,
    pub last_seen: Option<u64>,      // Unix seconds (disconnect timestamp)
    pub exit_node: bool,
    pub exit_families: ExitFamilies, // ?, 4, 6, d, n
}
```

### ExitFamilies (wire-optimized)

| Variant | Wire | Meaning |
|---|---|---|
| Unknown | "?" | No claim (or older coordinator) |
| V4 | "4" | IPv4 egress only |
| V6 | "6" | IPv6 egress only |
| Dual | "d" | Both families |
| Neither | "n" | No uplink available |

**Why one-char?** msgpack writes unit variants as their name. `Unknown` spelled out costs 8 bytes per member on the wire. One-char saves ~7 bytes × roster size.

### Wire Format

- **Posititional msgpack**: Field order IS the wire format
- **Array-encoded**: `Vec<Member>` serialized as plain array
- **Forward compatible**: New fields appended at end
- **Absent ≠ unknown**: A missing slot means "older roster was shorter"
