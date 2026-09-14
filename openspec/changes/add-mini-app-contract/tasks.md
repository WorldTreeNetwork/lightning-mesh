# Tasks

- [ ] Advise review of this change and `design.md` (reader sol-arch-review, other family); accept before code
- [x] Owed from advise: amend `canEmbed` and the sandbox requirement to refuse every key-bearing hello origin, including gateway-IP origins not equal to the current page origin; add the `http://hello.mesh` parent → `http://10.42.<x>.1` raw-entry regression scenario
- [x] Owed from advise: decide whether manifest discovery occurs before visitor intent; either defer it or document the app-host disclosure, narrow the tap-to-load privacy claim, and add an observable timing scenario (steer 2026-09-13: node-side fetch; the browser contacts no app host before the visitor opens it)
- [ ] `crates/mjolnir-hello`: bounded manifest + icon fetcher (mesh ranges only, Host header, no redirects, 3 s, 128 KiB, 5 min refresh, 1 h last-good) and `GET /api/apps`, with tests for every manifest scenario
- [ ] `crates/mjolnir-hello`: Rust manifest validation that mirrors `parseManifest` field-for-field (shared JSON fixtures drive both test suites)
- [x] Owed from advise: define the 16 KiB bridge limit as UTF-8 JSON bytes with serialization failure closed, and require a finite numeric height before clamping
- [ ] `hello-mesh-web/src/lib/miniapp/contract.ts`: `isMiniApp`, `appPath`, `entryUrl`, `entryOrigin`
- [ ] `contract.ts`: `parseManifest` (field limits, text-only values, defaults)
- [ ] `contract.ts`: `canEmbed(entryOrigin, hostOrigins)` and `FRAME_SANDBOX` / `FRAME_ALLOW` constants
- [ ] `contract.ts`: bridge envelope types, `acceptBridgeMessage(event, frameWindow, entryOrigin)`, `clampHeight`, `safeOpenUrl`
- [ ] `contract.spec.ts`: one test per scenario in `specs/mesh-mini-apps/spec.md`
- [ ] Directory fixture gains an app-marked service and a same-origin trap service
- [ ] Propose a "Publishing a mini-app" section for `docs/deploy/mesh-app-publishing.md` and get Duke's sign-off before editing it
- [ ] `bun run check` and `bun run test` green
