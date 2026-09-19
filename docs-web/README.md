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

Join pages (`docs/join/`) keep YAML frontmatter (`id`, `status`, `next_step`).
Relative `.md` links are rewritten to site paths. Archive and sprint notes
are omitted from the nav.
