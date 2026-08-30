# Lightning Admin

Operator desktop for [Lightning Mesh](../README.md). Same shape as MikroTik
Winbox / UniFi Network: sit on a LAN, discover the boxes, then administer
them. Built like [Papyrus](https://github.com/): Tauri 2 wrapping a SvelteKit
static SPA (`adapter-static` + `ssr = false`).

v1 of the window is discovery only — every IPv6 link-local (`fe80::`) address
on every local interface, plus neighbors from the kernel table and an ICMPv6
echo to `ff02::1` (all-nodes) scoped per interface.

## Run

```bash
cd admin
bun install
bun run tauri dev
```

Quality gates (no window needed):

```bash
bun test
bun run check
cargo test --manifest-path src-tauri/Cargo.toml
```

The frontend talks to Rust only through `invoke('scan_link_local')` — same
method as Papyrus `list_vms` / `list_boxes`.
