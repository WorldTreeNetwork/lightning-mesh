# Tasks

Authoritative owed work is bead `mjolnir-mesh-ncy.2`.

- [x] Human activation (banner to ACTIVE BUILD) — Duke 2026-09-15
- [ ] Advise by an independent non-Claude reader; accept before act
- [ ] Advise send-back: reconcile the proposed 256-byte path ceiling with the living consumer requirement and `contract.ts`; either remove it or modify and test both sides as one rule set
- [ ] Advise send-back: specify and test omitted `app.path` defaulting canonically to `/`, including the name-claim wire/storage representation and `--app-path` requiring `--app`
- [ ] `mjolnir-mesh`: shared `validate_app_marker` plus JSON vectors in `hello-mesh-web/src/lib/miniapp/fixtures/app-marker/`, also run by `contract.spec.ts`
- [ ] mjolnir-hello: optional `app` on `NameClaimRequest` and `NameClaimRecord`; reject invalid before spooling; routes tests (valid, invalid path, unsupported version, signature unchanged with and without `app`)
- [ ] meshd: carry `app` through the spool, leased-name record (`#[serde(default)]`) and directory TXT projection; renewal without `app` clears it; unit tests
- [ ] meshd control API: validate TXT `app`/`path` in `publish_service`; `400` with reason; CLI `--app` and `--app-path`
- [ ] `CARGO_TARGET_DIR=/tmp/lm-target cargo test -p mjolnir-hello` and `cargo test -p mjolnir-mesh --lib --features daemon` in the cross container; `bunx vitest run --project server src/lib/miniapp`
- [ ] Docs: `docs/join/publish/01-publish-a-service.md` and `docs/deploy/mesh-app-publishing.md` show `app` on name claims and `--app` on the CLI
