# add-app-shelf

> **ACTIVE BUILD**

Bead `mjolnir-mesh-ncy.3` (epic `mjolnir-mesh-ncy`). Laptop-only activation
2026-09-15. The contract is already living (`mesh-mini-apps`). This change
adds the hello.mesh Apps shelf that consumes it.

## Why

hello.mesh lists services as links. Mini-app records and `GET /api/apps`
exist, but visitors still have no tiles, tap-to-load cards, or link-out
fallback. Without a shelf, the folded contract is unused.

## What

- An Apps panel on the hello.mesh front desk, above Services.
- Tiles from directory services that `isMiniApp`, decorated by `/api/apps`.
- Tap-to-load sandboxed card when `canEmbed`; otherwise persistent open-in-tab.
- Empty, loading, and manifest-failure (link-out) states.
- Reuse `hello-mesh-web/src/lib/miniapp/contract.ts`. No second parser.

## Impact

- Capabilities: MODIFIED `mesh-mini-apps` (ADDED Apps shelf requirement).
- ADRs: none (insertion/sandbox already folded).

## User journey & surfaces

A visitor opens `http://hello.mesh/`. Apps shows tiles without contacting
app hosts. They tap a card-mode app: a sandboxed iframe loads the entry URL
and an open-in-new-tab control stays visible. A failed manifest is still a
tile that only links out. Ordinary unmarked services stay in Services.

## Out of scope

- Identity over the bridge: `add-embedded-assert` (`ncy.4`).
- `meshd publish --app`: `add-app-manifest-publish` (`ncy.2`, PENDING).
- mesh-app.js SDK: `ncy.5`.
- Live fleet deploy, HTTPS aliases, radio/SSID changes.
