# Tasks

- [ ] Advise review of this change and `design.md` (reader Fable 5.1); accept before code
- [ ] `hello-mesh-web/src/lib/miniapp/contract.ts`: `isMiniApp`, `appPath`, `entryUrl`, `entryOrigin`
- [ ] `contract.ts`: `parseManifest` (field limits, text-only values, defaults)
- [ ] `contract.ts`: `canEmbed(entryOrigin, hostOrigins)` and `FRAME_SANDBOX` / `FRAME_ALLOW` constants
- [ ] `contract.ts`: bridge envelope types, `acceptBridgeMessage(event, frameWindow, entryOrigin)`, `clampHeight`, `safeOpenUrl`
- [ ] `contract.spec.ts`: one test per scenario in `specs/mesh-mini-apps/spec.md`
- [ ] Directory fixture gains an app-marked service and a same-origin trap service
- [ ] Propose a "Publishing a mini-app" section for `docs/deploy/mesh-app-publishing.md` and get Duke's sign-off before editing it
- [ ] `bun run check` and `bun run test` green
