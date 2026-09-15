# add-app-manifest-publish

> **PENDING**

Bead `mjolnir-mesh-ncy.2` (epic `mjolnir-mesh-ncy`). Re-scoped by steer
2026-09-15: publishing must not require SSH, so the mini-app marker has to be
accepted wherever a name is accepted, not only through the router CLI. Builds on
[`add-mini-app-contract`](../add-mini-app-contract/proposal.md) (merged
`503cd17`, not yet folded).

## Why

The mini-app contract says a service is a mini-app when its TXT carries
`app=v1` and an optional `path=`. Today only the SSH-only operator command can
set TXT (`mjolnir-meshd publish --txt`). Key-owned names claimed by apps
themselves through `POST /api/name-claim` carry `scheme` and `ip` but have no
way to say "I'm a mini-app". So the only self-serve, SSH-free publishing path
can't produce a mini-app at all. And nothing validates the marker at publish
time, so a bad `path` fails silently on the shelf.

## What

- **Name claims.** `POST /api/name-claim` accepts an optional, self-reported
  `app` object: `{"v": 1, "path": "/entry"}`.
  - Like `ip` and `scheme`, it's outside the signed v1 preimage, so the
    shipped ceremony is unchanged.
  - hello rejects an invalid marker with `400`.
  - meshd carries it into the leased-name record, and the directory projects it
    as TXT `app=v1` plus `path=`.
- **Control API.** `POST /v0/publish` (and so `mjolnir-meshd publish`)
  validates `app`/`path` in TXT with the same rules, and rejects invalid ones
  with `400` and a reason. The CLI gains `--app [--app-path /x]` as a
  convenience. It remains the recovery path, not the headline.
- **One validator** shared by both paths, matching `contract.ts` `appPath` and
  the shared fixtures, so publishers get the same answer the shelf will.
- **Consumers** (`/api/directory`, `/api/apps`, the browser contract) still
  re-validate. Publish-time rejection is feedback, not the security boundary.

## Impact

- Capabilities: ADDED to `mesh-mini-apps` (the capability is added by
  `add-mini-app-contract`, and this change's requirement folds after it).
- ADR: `design.md` (unsigned marker, and moving it into owner-signed records
  later).
- Code: `crates/mjolnir-hello/src/routes.rs` (`NameClaimRequest`,
  `NameClaimRecord`); meshd name-claim spool, leased-name record and directory
  projection (`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`, `crdt/leased_name.rs`);
  `publish_service` and the CLI.
- Wire: the leased-name CRDT record gains an optional field. The fleet upgrades
  together, per the no-mixed-fleet rule, but older records without it must
  still parse.

## User journey & surfaces

An app developer runs walkie-talkie on a Pi with its own key.

1. **Working.** Its renew loop adds `"app":{"v":1,"path":"/"}` to each name
   claim. Within about 5 seconds the directory lists `walkie-talkie` with TXT
   `app=v1`, `path=/`, and `/api/apps` fetches its manifest. It shows on the
   Apps shelf once `add-app-shelf` ships. No SSH was used.
2. **Empty.** A claim without `app` behaves exactly as today: a plain service,
   clickable if it has a `scheme`.
3. **Failed.** `"path":"//evil.example"` → `400 {"error":"invalid app path"}`.
   Nothing is spooled and the previous lease is untouched. An operator running
   `mjolnir-meshd publish foo --txt app=v2` gets `400 unsupported app version`.
4. **Off.** Routers running an older build ignore the unknown field. The name
   still resolves as a plain service.

## Out of scope

- Signing the marker: it moves into owner-signed name records when those land
  (`ai0.9` follow-on).
- Publishing through the signed admin RPC (`b6j.2`): that change calls this
  validator.
- Apps shelf UI: `add-app-shelf` (`ncy.3`).
- Manifest fetching: already built in `add-mini-app-contract`.
