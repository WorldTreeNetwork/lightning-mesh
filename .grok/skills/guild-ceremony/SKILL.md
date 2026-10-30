---
name: guild-ceremony
description: >
  Sign an Admin house guild and a named workplace guild, assign a node,
  and read the local log back. Use when the user asks for an admin group,
  a workplace group, a guild, its id, or to add a node. Run lightning-guild
  list, show, assign, or ceremony. The order is admin/scripts/guild/ceremony.rhai.
---

# Guild ceremony

Signer and reader: `admin/src-tauri/src/bin/lightning-guild.rs`.
Order: `admin/scripts/guild/ceremony.rhai`.
Bodies: `admin/src-tauri/src/guild.rs` (`sign_guild_at`).
View: `admin/src-tauri/src/guild_view.rs` replays the log. Do not write a second signer or a second decoder.

Product word is guild. Wire kinds are `keyspace.*`. A display name is not a protocol field. There is no rename op. Vocabulary: `docs/glossary.md`.

The human guild id is base58 of the 32-byte keyspace content hash. That id is not an XID. An XID is the hash of an inception signing key for an identikey document, and this ceremony does not create one.

## Look

```bash
lightning-guild
lightning-guild show "Strawberry Creek"
lightning-guild show <base58-or-hex-prefix>
lightning-guild --json show "Strawberry Creek"
```

`you admin` means the actor seed on this machine is `admin: true` on that guild's roster and every log line verified. The replica is the local log. These ops are not on the router.

## Change

```bash
lightning-guild assign --guild "Strawberry Creek" --node-name <name> --node-id <64-hex>
lightning-guild ceremony --workplace NAME --node-name NAME --node-id HEX
lightning-guild ceremony --workplace-id HEX --node-name NAME --node-id HEX
```

`assign` resolves the guild by name, base58 id, or hex prefix, then appends `assign-node` and `nest`. Genesis is still `ceremony` with `--workplace`, and only while the log is empty.

## Steps

1. Read the node id from the node (`service mjolnir-meshd diag`, or `GET /api/node`). It is 64 hex characters. Copy it. Do not invent one.
2. Update this repo (`git pull --rebase` on a clean tree; stash unrelated dirt and pop it after). Build the signer from that tree.
3. Build:

```bash
CARGO_TARGET_DIR=/tmp/lm-admin-target \
  cargo build --manifest-path admin/src-tauri/Cargo.toml --bin lightning-guild
```

`target/` at the repo root is often root-owned. The admin crate is not a workspace member.

4. Empty log: genesis. This signs four ops in one millisecond, counters 0–3:

- `keyspace.join` with no guild id → Admin house guild. The actor is `admin: true`. The keyspace id is that op's content hash.
- `keyspace.join` again → workplace house guild. Same actor, admin. Id is that op's content hash.
- `keyspace.assign-node` of the node id onto the workplace guild.
- `keyspace.nest` under the workplace guild, profile `access-node`. The access-node id is the nest op's content hash.

```bash
/tmp/lm-admin-target/debug/lightning-guild ceremony \
  --workplace "Strawberry Creek" \
  --node-name ap3000-outdoor-b \
  --node-id <64-hex-node-id>
```

5. Log already has ops: do not genesis again. Assign the next node onto the workplace id from `guilds.json`:

```bash
/tmp/lm-admin-target/debug/lightning-guild ceremony \
  --workplace-id <workplace-keyspace-id> \
  --node-name <name> \
  --node-id <64-hex-node-id>
```

That appends `assign-node` and `nest` only.

6. Stdout is one JSON object: actor public key, admin guild id, workplace guild id, access-node guild id, and the three file paths. The seed is not in that JSON.

## Files

All under `~/.config/lightning-admin/` (or `LIGHTNING_GUILD_CONFIG`):

| File | Mode | Contents |
|---|---|---|
| `actor-seed` | 0600 | 32-byte Ed25519 seed, 64 hex chars. Created once if missing. |
| `guild-ops.log` | 0600 | One signed op per hex line. Source of truth. |
| `guilds.json` | 0600 | Local names: Admin, the workplace, and each node's access-node id. |

The config directory is mode 0700. Never print the seed, put it on a command line, or commit these files.

## Leave alone

- Wraps stay empty. This ceremony does not mint a guild PTK.
- Signing does not change the client SSID and does not SSH the op to the router.
- A second Admin genesis is refused while the log is non-empty.
