# add-house-ula-prefix

> **ACTIVE BUILD**

## Why

Every install currently shares one compiled ULA `/48`, so two houses do not have distinct prefixes and a node cannot yet be named by a house-scoped `/128`. The backhaul campaign needs that pure function before forwarding, Admin's TUN, or a foreign-prefix admit can mean anything.

## What

- Modify `identity-derived-ula`.
- Keep the compiled-constant `/64` on `br-mesh` (the address live nodes already have).
- Add a house `/48` derived from the house guild id, a router `/128` in subnet 0, and an app `/128` in subnet 1.
- When `/etc/mjolnir/house-guild-id` holds the guild's 32-byte id, assign the router `/128` beside the legacy `/64` and publish it as `house_ula`.
- Leave Babel's `fc00::/7` deny and the `10.254` Iroh pin alone.

## Impact

- Capabilities: MODIFIED `identity-derived-ula`
- ADRs: none. `ARCHITECTURE.md` stays until forwarding folds the shape change.

## User journey & surfaces

No new UI because Admin's address list and SSH instructions still show the legacy ULA. `GET /api/node` gains `house_ula`, omitted when the guild file is absent. Rendering that address as the node reference is `add-ula-node-reference`.

## Out of scope

- Babel host routes and narrowing the legacy `/64` (`add-ula-host-forwarding`, `mjolnir-mesh-bny6.6`).
- Pointing operators at the new address (`add-ula-node-reference`).
- Foreign `/48` admission (`add-foreign-prefix-permit`).
- Admin TUN (`add-admin-app-tun`).
- Removing `10.254` (`remove-v4-backhaul-pin`).

## Preparation

- Upstream: none. This node has no inbound dependency.
- Anchors: `openspec/specs/identity-derived-ula/spec.md`; `crates/mjolnir-mesh/src/tun/link.rs` `ula_prefix` / `ula_addr`; `assign_ula_addr` in `mjolnir-meshd.rs`; `DirectoryNode.ula`.
- Outcome: revised again after `reviews/2026-10-31-advise-r2.md` (send-back). Restart ownership is `/var/lib/mjolnir/house-ula`. Usable means present, not tentative, and not DAD-failed. Fresh advise is still owed.
- Authority: campaign grant on `mjolnir-mesh-bny6` ("activate and run"), bounds are that epic's non-goals and the recorded steer.
