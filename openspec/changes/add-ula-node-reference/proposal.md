# add-ula-node-reference

> **ACTIVE BUILD**

**Rigor:** change

## Why

The house router `/128` is already assigned and forwarded, and `/api/node` already returns it as `house_ula` for this node. The directory neighbor list, the Admin address list, and the fleet inventory still present `10.254` as the only backhaul reference. An operator on another site of the same house cannot see the address the host route actually uses.

## What

- A directory neighbor gains `house_ula`, computed with `router_ula` from this node's guild file and that neighbor's node id. `addrs` stays the transport pin.
- Admin's address list shows that `/128` as the backhaul reference and keeps `10.254` visible as the transport pin.
- The fleet inventory keeps the `backhaul_addr` column as `10.254` and prints the house `/128` beside it from the same function. No second hash.

## Impact

- Capabilities: ADDED identity-derived-ula
- ADRs: none

## User journey & surfaces

An operator on one site opens Lightning Admin's address list, or the directory neighbor entry, and reads the other site's house `/128` in Rust's `Display` form. `10.254` is still on the row, labeled as the transport pin. The fleet inventory print shows the same pair. SSH to that `/128` is the host route `add-ula-host-forwarding` already installed; this node does not open a radio.

Working: guild file valid, neighbor node id known, `house_ula` equals `router_ula`. Empty: no guild file, the field is omitted and the row keeps `10.254`. Failed: a neighbor with no node id is unchanged.

## Out of scope

- Replacing `DirectoryNode.backhaul_addr` or the fleet column that update scripts walk
- A new name system, DNS, or AAAA
- Binding Iroh to the `/128` (`remove-v4-backhaul-pin`)
- A mesh errand, a fleet deploy, or a live SSH check
- Babel filter changes (`add-router-admitted-ula`)

## Preparation

- Upstream: `add-ula-host-forwarding` folded at `634f296`. `GET /api/node` already includes `house_ula` on the local node (`DirectoryNode` in `mjolnir-meshd.rs`). `router_ula` Display for guild `11` repeated and node id `node-example` is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`. Admin drops `house_ula` in `admin/src-tauri/src/net/hello.rs`. The address list prints `meshBackhaul` in `admin/src/routes/+page.svelte`. Fleet columns are `name|backhaul_addr|node_id|model|notes` in `deploy/openwrt/fleet-file.sh`.
- Outcome: revised. No prior change directory. The contract points those three surfaces at the existing `/128` and leaves `10.254` in place.
- Authority: 2026-11-01 "activate and run" for campaign `mjolnir-mesh-bny6`. Bounds: packet-only; do not join the client SSID; do not edit the Babel reconcile that `add-router-admitted-ula` owns.
- Write set overlaps `mjolnir-meshd.rs` with `add-router-admitted-ula`. Not one wave. This node edits directory neighbor construction only.
