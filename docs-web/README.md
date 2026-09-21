# Lightning Mesh docs site

Server-side generated static SvelteKit app for the public user manual in
`content/`. This package prerenders the Markdown as HTML at build time
(`adapter-static`).

```sh
cd docs-web
bun install
bun run dev      # http://localhost:5173
bun run build    # writes docs-web/build/
bun run preview
```

`docs-web/content/` is the publication boundary: Markdown there is public;
architecture, research, sprint notes, and maintainer references stay in
`docs/` and are never scanned by the site build. Guide pages keep YAML
frontmatter (`id`, `status`, `next_step`). Relative `.md` links inside the
guide become site paths; links that escape `content/` go to GitHub.

## Live site (IdentiKey Sites)

Public URL: [https://lightning.worldtree.network/](https://lightning.worldtree.network/)

This is **Sites**, not `mj deploy`. Build locally, upload `build/`. The
signing key is `~/.config/mjolnir/identikey.json` (do not commit it).

```sh
cd docs-web
./scripts/publish.sh
```

|                       |                                                             |
| --------------------- | ----------------------------------------------------------- |
| Site name             | `lightning-mesh`                                            |
| IdentiKey fingerprint | `9m9YdqCFcQKGtLzAk2rKYMLK9xw9zERpJiEqWk3NAfaQ`              |
| Domain                | `lightning.worldtree.network` (A → `45.76.77.97`, DNS-only) |
