# Tasks

- [x] Advise review of this change (other family); accept before code
- [x] `hello-mesh-web/src/lib/miniapp/apps.ts`: directory filtered by `isMiniApp`; `/api/apps` lookup; failed/unloaded response → link-out tiles
- [x] `AppsPanel.svelte`: tiles, tap-to-load card using `FRAME_*` + `canEmbed` + `window.location.origin`, one card at a time, remove iframe+listener on close, bridge ready/init/resize/open, persistent open-in-tab, compact empty
- [x] Mini-apps omitted from Services
- [x] `+page.svelte` mounts Apps above Services
- [x] vitest: join/fallback, stale keeps embed, canEmbed link-out, listener teardown
- [x] Playwright: fixture app tile → card iframe; failed manifest → link-out; no iframe/`<link>` to app hosts on load
- [x] `bun run check` and `bun run test` green
