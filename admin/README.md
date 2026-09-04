# Lightning Admin

Operator desktop for [Lightning Mesh](../README.md). Same shape as MikroTik
Winbox / UniFi Network: sit on a LAN, discover the boxes, then administer
them. Built like [Papyrus](https://github.com/): Tauri 2 wrapping a SvelteKit
static SPA (`adapter-static` + `ssr = false`).

v1 of the window is discovery only — Unique Local (`fc00::/7`) and link-local
(`fe80::`) IPv6 on every local interface, plus neighbors from the kernel table
and an ICMPv6 echo to `ff02::1` (all-nodes) scoped per interface.

The list shows each address as **base58 of the 16 IPv6 octets** (one hex
couplet = one byte). It defaults to Unique Local (`fc00::/7`); switch to
LL or All in the header. Hover a base58 to read the canonical IPv6; click
copies base58, the hover panel copies IPv6, and **Copy list** dumps the
visible rows as `base58<TAB>ipv6<TAB>iface` for pasting.

`bun run dev` then open `http://localhost:1420/?demo=1` to preview the
list without the Tauri shell.

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
