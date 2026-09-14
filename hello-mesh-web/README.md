# hello-mesh-web

The **hello.mesh front desk** web app: the page every Lightning Mesh router
serves at `http://hello.mesh` (and at its LAN gateway `http://10.42.<x>.1`).
It shows the People / Services / Routers directory, creates and restores the
browser-held IdentiKey, and hosts the cross-origin sign-in page (`/assert`).

SvelteKit + TypeScript, built as a static site (`@sveltejs/adapter-static`)
and embedded into the `mjolnir-hello` router binary. The page is served over
plain HTTP, so it can't rely on WebCrypto or other secure-context APIs.

User-facing guide: [`docs/join/person/`](../docs/join/person/02-hello-mesh.md).
Server and daemon seams: `crates/mjolnir-hello`,
[`docs/network-coordination/hello-mesh-service.md`](../docs/network-coordination/hello-mesh-service.md).

## Develop

Package manager is **bun**.

```sh
bun install
bun run dev          # vite dev server
bun run check        # svelte-check
bun run test         # vitest, once
bun run test:smoke   # Playwright smoke tests (e2e/)
bun run lint
```

## Build and ship

```sh
bun run build:embed  # vite build, then scripts/sync-embed.js copies build/
                     # into crates/mjolnir-hello/static/
```

`mjolnir-hello` bakes `crates/mjolnir-hello/static/` into the binary at compile
time (rust-embed), so the web build must run **before** the Rust build or a
stale page ships. For routers, don't run these steps by hand; use
`deploy/openwrt/build-hello.sh`, which runs `build:embed` and then
cross-compiles the aarch64 binary. See `deploy/openwrt/README.md`.
