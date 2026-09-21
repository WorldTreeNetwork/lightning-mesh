---
id: contribute
title: Contribute
description: License, the contributor agreement, how work is tracked, and where to ask for help.
path: contribute
order: 1
audience: [person, operator, developer, agent]
status: built
time: reference
requires: []
next_step: null
verified_against: 503cd17 (2026-09-13)
---

# Contribute

## License

Lightning Mesh is free software under **AGPL-3.0-or-later**, with a
commercial license available for closed products. See
[`LICENSE`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/LICENSE)
and
[`COMMERCIAL-LICENSE.md`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/COMMERCIAL-LICENSE.md).

## Before your first pull request

Agree to the
[Contributor License Agreement](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/CLA.md).
A bot comments on your first PR with a one-click sign link. You keep your
copyright; the CLA is a license, not an assignment. Contact details for
corporate agreements and security reports are in
[`CONTRIBUTING.md`](https://github.com/WorldTreeNetwork/lightning-mesh/blob/main/CONTRIBUTING.md).

## How work is tracked

The project uses **beads** (`bd`), not GitHub issues:

```sh
bd ready          # work with no blockers
bd show <id>      # details of one item
bd update <id> --claim
```

Reference the bead id in your pull request.

## Development

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo fmt --all
```

The router daemon only builds for Linux. Use `deploy/openwrt/build.sh`
(Docker) from macOS.

## Editing this guide

This guide lives in
[`docs-web/content/join/`](https://github.com/WorldTreeNetwork/lightning-mesh/tree/main/docs-web/content/join).
Every page has YAML frontmatter:

| Field | Meaning |
|---|---|
| `id` | Stable page id, used in `next_step` and `requires` |
| `path` | `house`, `person`, `node`, `publish` or `contribute` |
| `status` | `built`, `partial` or `coming-soon`, matching what's deployed |
| `requires` / `next_step` | Reading order for people and agents |
| `verified_against` | Commit the steps were checked against |

Steps use the same shape everywhere: **Do**, **Expect**, **If not**.
When behaviour changes, update the step and `verified_against` in the same
commit. Preview with `cd docs-web && bun run dev`. Publish with
`docs-web/scripts/publish.sh` → https://lightning.worldtree.network/.
