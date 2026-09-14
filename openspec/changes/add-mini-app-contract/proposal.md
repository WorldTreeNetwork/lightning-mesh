# add-mini-app-contract

> **ACTIVE BUILD**

Bead `mjolnir-mesh-ncy.1` (epic `mjolnir-mesh-ncy`). Human activated 2026-09-13.
Steer (2026-09-13, Duke took the recommended defaults): sandboxed
cross-origin iframe card with link-out fallback; TXT marker plus a
`/.well-known` manifest; identity rides the bridge (`add-embedded-assert`).
Stage-1 surface is hello.mesh only.

## Why

Services already publish (`meshd publish` → `ServiceBookV2` → gossip →
`services[]`) and hello.mesh already lists them as links. Nothing says a
service is an *app*, nothing lets hello.mesh show one in place, and there is
no agreed way for the two pages to talk. Every future mini-app inherits
whatever we pick here, and the wrong insertion model puts app script in the
origin that holds the visitor's identity key. Settle the contract once.

## What

- **Marker.** A service is a mini-app when its TXT carries `app=v1`;
  optional `path=` names the entry path. Works with today's
  `meshd publish --txt`, no CRDT schema change.
- **Manifest.** hello.mesh reads `<entry origin>/.well-known/mesh-app.json`
  from the browser for name, description, icon, embed mode and height.
  Missing or bad manifest degrades to a link tile.
- **Insertion.** `embed: card` renders a tap-to-load sandboxed iframe on the
  app's own origin; otherwise, or whenever the entry origin equals a
  hello.mesh origin, the tile links out. No app script, markup or style
  ever enters the hello.mesh document.
- **Bridge.** A versioned `postMessage` envelope (`mini-app/v1`) with
  origin-pinned send and receive, and v1 types `ready`, `init`, `resize`,
  `open`. Identity types are added by `add-embedded-assert`.
- Lands the shared pure module `hello-mesh-web/src/lib/miniapp/contract.ts`
  that `add-app-shelf`, `add-embedded-assert` and `add-mesh-app-sdk` import.

## Impact

- Capabilities: ADDED `mesh-mini-apps`
- ADRs: `design.md` in this change (security boundary: key origin vs app
  origin)

## User journey & surfaces

No new UI because this change lands the contract module and its tests; the
Apps shelf that renders it is `add-app-shelf` (`mjolnir-mesh-ncy.3`). The
outcome is reached through `openspec/specs/mesh-mini-apps/spec.md` and
`hello-mesh-web/src/lib/miniapp/contract.ts`, which that shelf consumes.
The existing Services panel is unchanged and keeps listing every service,
apps included.

## Out of scope

- Apps shelf UI and states — `add-app-shelf` (`ncy.3`)
- `meshd publish --app` convenience and publish-time validation —
  `add-app-manifest-publish` (`ncy.2`)
- Identity over the bridge — `add-embedded-assert` (`ncy.4`)
- `mesh-app.js` SDK and a reference app — `add-mesh-app-sdk` (`ncy.5`)
- Apps on the captive-portal page or in Lightning Admin — later stage
- Self-serve app publishing without an operator — `mjolnir-mesh-8tk`
- Expiring dead apps — `mjolnir-mesh-e21.9`
- Push instead of the 5s directory poll — `mjolnir-mesh-9vb`
