# extract-mjolnir-app-tun

> **ACTIVE BUILD**

## Why

The app TUN session lives inside Lightning Admin. Another desktop app cannot open the same `/128` without compiling Admin or the router crate. The address math and the datagram seam are already separate from Iroh. The session should be too.

## What

- Add workspace crate `mjolnir-app-tun` underneath `mjolnir-mesh` and Admin.
- Move `house_ula_prefix`, `app_ula`, `DatagramConn`, `spawn_encap_pair`, and the Linux TUN session into that crate.
- Admin keeps the guild window and calls the crate. Foreign `/48` sync takes prefix bytes, not Admin permit types.
- No crates.io publish in this change.

## Impact

- Capabilities: ADDED mjolnir-app-tun; MODIFIED admin-app-tun; MODIFIED admitted-prefix-route
- ADRs: none

## User journey & surfaces

No new UI because the guild surface already shows the app address and still does not open a TUN. A second process, or another app, calls the crate with a guild id, an app public key, and one `DatagramConn`.

## Out of scope

- Publishing `mjolnir-app-tun` to crates.io (later bead)
- Babel allows and overlay demux (`add-ula-host-forwarding`, spike `overlay-ula-demux`)
- Router export of admitted prefixes (`add-router-admitted-ula`)
- Moving the Iroh dial pin off `10.254` (`remove-v4-backhaul-pin`)
- Linking Iroh from the new crate

## Preparation

Upstream: `add-admin-app-tun` folded in `57f7e2c`, `add-admitted-prefix-route` folded in `6054dc0`. Living specs `openspec/specs/admin-app-tun/spec.md` and `openspec/specs/admitted-prefix-route/spec.md`. Code anchors `admin/src-tauri/src/app_tun.rs`, `crates/mjolnir-mesh/src/tun/link.rs`, `crates/mjolnir-mesh/src/tun/encap.rs`. Outcome: new contract, steered 2026-11-01. Authority: the user added `mjolnir-mesh-bny6.9` to epic `mjolnir-mesh-bny6` and said to continue. Bounds: crate underneath, in-repo only, no Iroh, guild window stays in Admin. The demux spike was not confirmed and is not this node.
