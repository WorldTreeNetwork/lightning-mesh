# add-router-admitted-ula

> **ACTIVE BUILD**

**Rigor:** architecture

## Why

Babel on `br-mesh` and on `mjolnir0` already exports this house's router `/128` and allows that house `/48` at prefix length 128. An admitted foreign house is still inside the `fc00::/7` deny, so its host routes never cross either Babel. A covering prefix would make the foreign `/48` on-link. The foreign prefix has to be the same kind of host allow, and nothing else.

## What

- Both Babel renders take the caller-supplied admitted `/48` list and allow each one with `eq 128` before the `fc00::/7` denies.
- The list is the projection file `/var/lib/mjolnir/admitted-prefixes`. The renderer does not read the guild log and does not decide that a prefix is admitted. The publisher guarantees each line is an admitted foreign house `/48`. The reader does not.
- The reader counts only an `fd00::/8` `/48`. It skips `ula_prefix()` (`fd53:6213:4797::/48` for this build) and the house `/48` from the guild file, even when that house `/128` is not usable. It does not keep a denylist of other stock prefixes.
- A missing file clears the list. An unreadable file keeps the last good read. A failed Babel restart stays pending and is retried on the next pass of both reconcilers.
- A foreign prefix is not assigned on an interface and is not redistributed as a local address. `10.254` stays the Iroh bind. IPv4 Babel next hops stay link-local.

## Impact

- Capabilities: MODIFIED identity-derived-ula
- ADRs: none

## User journey & surfaces

No new UI because an admitted `/128` becomes a Babel host route on the routers that already speak Babel. The operator surface that shows the address is `add-ula-node-reference`.

## Out of scope

- Re-checking `keyspace.admit-prefix` signatures inside `mjolnir-meshd` (the permit stays in Admin; this node only reads the projection)
- Installing the foreign `/48` as a TUN device route (`admitted-prefix-route` already does that on the app TUN)
- Binding or dialing Iroh on `fd` (`remove-v4-backhaul-pin`, spike `iroh-ula-dial` still open)
- Writing the projection file from Admin (a later publisher; this node defines the reader and the render)
- Phones, AAAA, client-LAN RA, and the `10.42` pool

## Preparation

- Upstream: `add-ula-host-forwarding` folded at `634f296`; `add-foreign-prefix-permit` and `add-admitted-prefix-route` folded. Living requirement is `openspec/specs/identity-derived-ula/spec.md` ("ULA sits beside 10.254"). Code anchors: `house_host_filters` in `crates/mjolnir-mesh/src/babel/config.rs`, `known_house_router` in `crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`.
- Outcome: revised. No prior change directory. The contract adds the admitted list beside the landed house allow and does not change the Iroh bind.
- Authority: 2026-11-01 "activate and run" for campaign `mjolnir-mesh-bny6`. Bounds: do not take the epic; do not act this node before an other-family accept; do not act `remove-v4-backhaul-pin` before `iroh-ula-dial` goes; do not commit `openspec/changes/add-zero-touch-join/tasks.md` or `.beads.gate.lock`.
- Write set overlaps `mjolnir-meshd.rs` with `add-ula-node-reference` (directory neighbor versus Babel reconcile). Not one wave.
