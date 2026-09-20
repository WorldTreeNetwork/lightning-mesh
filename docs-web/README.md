# Lightning Mesh docs site

Server-side generated static SvelteKit app for `docs/`. Markdown stays in
the repo; this package prerenders HTML at build time (`adapter-static`).

```sh
cd docs-web
bun install
bun run dev      # http://localhost:5173
bun run build    # writes docs-web/build/
bun run preview
```

Only the user guide (`docs/join/`) is published. Architecture, vision, and
research stay in the repo. Join pages keep YAML frontmatter (`id`, `status`,
`next_step`). Relative `.md` links inside the guide become site paths; links
to other docs go to GitHub.

## Live site (IdentiKey Sites)

Public URL: [https://lightning.worldtree.network/](https://lightning.worldtree.network/)

This is **Sites**, not `mj deploy`. Build locally, upload `build/`. The
signing key is `~/.config/mjolnir/identikey.json` (do not commit it).

```sh
cd docs-web
./scripts/publish.sh
```

| | |
|---|---|
| Site name | `lightning-mesh` |
| IdentiKey fingerprint | `7PfGe1Dsx176UjgvTSaQd9Bc6hXBiCaJrNz3Tcd2noWL` |
| Domain | `lightning.worldtree.network` (A → `45.76.77.97`, DNS-only) |
