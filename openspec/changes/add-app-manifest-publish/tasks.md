# Tasks

Authoritative owed work is bead `mjolnir-mesh-ncy.2`.

- [x] Human activation (banner to ACTIVE BUILD) — Duke 2026-09-15
- [ ] Advise by an independent non-Claude reader; accept before act
- [x] Advise send-back: drop the 256-byte path ceiling so publish matches living `appPath` / `contract.ts` (pinned 2026-09-16)
- [x] Advise send-back: omitted `app.path` is `/`; `{"app":{"v":1}}` accepted; `--app-path` requires `--app` (pinned 2026-09-16; vectors on implement boxes)
- [x] Advise r2 send-back (Fable 2026-09-16): marker present iff TXT `app`; `path` validated only under `app`; plain `--txt path=print` still publishes (pinned 2026-09-17)
- [ ] `mjolnir-mesh`: shared `validate_app_marker` plus JSON vectors in `hello-mesh-web/src/lib/miniapp/fixtures/app-marker/`, also run by `contract.spec.ts`
- [ ] mjolnir-hello: optional `app` on `NameClaimRequest` and `NameClaimRecord`; reject invalid before spooling; routes tests (valid, invalid path, unsupported version, signature unchanged with and without `app`)
- [ ] meshd: carry `app` through the spool, leased-name record (`#[serde(default)]`) and directory TXT projection; renewal without `app` clears it; unit tests
- [ ] meshd control API: if TXT `app` is present, validate it and `path` with the shared function (`400` + reason); if `app` is absent, leave `path` and other TXT opaque; CLI `--app` and `--app-path`
- [ ] `CARGO_TARGET_DIR=/tmp/lm-target cargo test -p mjolnir-hello` and `cargo test -p mjolnir-mesh --lib --features daemon` in the cross container; `bunx vitest run --project server src/lib/miniapp`
- [ ] Docs: `docs/join/publish/01-publish-a-service.md` and `docs/deploy/mesh-app-publishing.md` show `app` on name claims and `--app` on the CLI
