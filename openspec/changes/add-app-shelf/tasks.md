# Tasks

- [ ] Advise review of this change (other family); accept before code
- [ ] `hello-mesh-web/src/lib/miniapp/apps.ts`: fetch `/api/apps`, join with directory services via `isMiniApp`
- [ ] `AppsPanel.svelte`: tiles, tap-to-load card using `FRAME_*` + `canEmbed`, persistent open-in-tab, empty/loading/failed
- [ ] `+page.svelte` mounts Apps above Services
- [ ] vitest: join/fallback, canEmbed link-out, no iframe before tap
- [ ] Playwright: fixture app tile → card iframe; failed manifest → link-out tile; page load makes no request to app hosts
- [ ] `bun run check` and `bun run test` green
