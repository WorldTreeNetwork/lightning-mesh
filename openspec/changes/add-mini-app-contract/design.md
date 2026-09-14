# Design: mini-app contract

## The boundary this protects

The visitor's rung-1 Ed25519 key lives in the IndexedDB of the hello.mesh
origin (`http://hello.mesh`, and separately `http://10.42.<x>.1` when the
page is loaded by gateway IP). Any script that executes in that origin can
read the key. So the one hard rule is: **app-supplied code never runs in a
hello.mesh origin.** Everything else here follows from that.

## Decisions

### 1. Insertion: cross-origin sandboxed iframe, link-out fallback

| Option | Verdict |
|---|---|
| Link-out tile only | Safe, but not "inserted"; kept as the fallback |
| **Iframe on the app's own origin** | Chosen. Browser origin isolation keeps the key out of reach; the app keeps its own storage and cookies |
| Web component / script loaded into hello.mesh | Rejected. Runs in the key's origin |
| Server-side HTML fragment injected into the page | Rejected. Same problem plus an XSS surface |

Sandbox: `allow-scripts allow-same-origin allow-forms allow-popups
allow-popups-to-escape-sandbox`. `allow-same-origin` is safe **only because**
the frame is cross-origin; it lets the app use its own storage. That is why
`canEmbed` refuses every **key-bearing** origin, not only the current page:
with a same-origin src, `allow-scripts` + `allow-same-origin` lets the frame
reach the parent and remove its own sandbox, and with a src at *another*
key-bearing origin, app code runs where a key may already be stored.

The key-bearing set is open-ended. Every node serves hello.mesh at its own
LAN gateway `http://10.42.<x>.1`, and a visitor may have minted a key on any
of them, so enumerating the set from the directory can't be complete.
The rule is therefore structural rather than enumerated: card embedding
requires a `.mesh` **name** host that isn't reserved. IP-literal hosts
(every gateway, and the IP fallback for dotted mDNS-style names) always
link out. Reserved names (`hello.mesh`, `id.mesh`) always link out. So does
the current page origin. Invariant for later work: any new name that serves
the hello.mesh page must join the reserved list. (Advise send-back
2026-09-13, sol-arch-review.) No `allow-top-navigation`; the app
asks the host to `open` instead. `allow=""` grants no camera, microphone,
geolocation or similar features in v1. `referrerpolicy="no-referrer"`.

Frames load only when the visitor taps the tile. The shelf does not preload
every app, for battery on phones and so a visitor's presence isn't announced
to every app host.

### 2. Discovery: TXT marker plus a well-known manifest

The gossiped record carries only `app=v1` and optional `path=`, a few bytes
per service on a CRDT that every node re-broadcasts. Presentation (name,
description, icon, embed mode, height) lives in
`/.well-known/mesh-app.json` on the app, so an app can restyle itself
without republishing and gossip stays small.

**Who fetches it: the node, not the visitor's browser** (steer 2026-09-13,
after advise flagged that a browser-side fetch announces every visitor to
every app host on page load).

| Option | Verdict |
|---|---|
| Browser fetches on load | Rejected. Every app host sees every visitor's IP, apps must send CORS, and self-signed https apps can't be read. |
| Browser fetches on tap | Rejected. Private, but tiles are bare names until tapped. |
| **Node fetches, caches, serves `/api/apps`** | Chosen. The browser talks only to hello.mesh origins until the visitor opens an app. No CORS burden, and self-signed https apps still get decorated. |

Shape in `mjolnir-hello` (sync `tiny_http`, and no HTTP client today):
- A background refresher thread reads the same cached `directory.json`
  projection that `/api/directory` serves (`routes.rs` `DirectoryCache`).
  It fetches each mini-app's manifest and icon and swaps an in-memory map
  that `GET /api/apps` serializes. No request path ever blocks on an app
  host.
- The client is `ureq` with rustls and a certificate verifier that
  **skips verification, used only by this fetcher** (manifest is
  presentation data, never authority). It follows no redirects, sets
  `Host` to the entry host, times out at 3 s and caps bodies at 128 KiB.
- Scope guard against making the router a general fetch proxy: only the
  record's own `ip` inside `10.42.0.0/16` or `10.254.0.0/16`, only the
  manifest path and the manifest-named same-origin icon path, `GET` only.
- Budget: the static aarch64 binary grows by the TLS stack. Measure it on
  the cross-build and record the delta in the act result. If it's
  unacceptable, drop to plain-HTTP-only fetch over `std::net::TcpStream`,
  and https apps link out undecorated.

Costs that remain:
- Decoration lags a manifest change by up to the 5-minute refresh.
- A self-signed https app still can't be **framed** from an `http`
  hello.mesh page until the visitor has accepted its certificate. The card
  shows the open-in-new-tab control, and that tab is where the visitor
  accepts it.

Manifest values are data. They render as text or as `<img src>`, never as
HTML. The manifest name is self-claimed, so any identity or consent surface
shows the entry **origin** as the authority (see `add-embedded-assert`).

### 3. Bridge: origin-pinned postMessage envelope

```json
{ "mesh": "mini-app/v1", "type": "resize", "height": 320 }
```

- Receive: accept only when `event.source === frame.contentWindow` **and**
  `event.origin === entryOrigin`. Both checks, because a frame can navigate
  itself to another origin.
- Send: always `postMessage(msg, entryOrigin)`, never `"*"`, so a navigated
  frame gets nothing.
- Drop any unknown `type`, and any message whose `JSON.stringify` throws
  (caught) or yields more than 16 KiB of UTF-8 bytes.
- Numeric fields must be finite numbers before use. One `clampHeight` serves
  both the manifest `height` and bridge `resize`.
- v1 types: app→host `ready`, `resize {height}`, `open {url}`; host→app
  `init {v: 1}`, sent once in response to the first `ready`.
- Heights clamp to 120–640 CSS px; `open` accepts only `http:` / `https:`
  and opens with `noopener`.

Versioning: a new envelope version is a new `mesh` string. The host ignores
versions it doesn't know, and the app can detect a missing `init`.

## Entry URL

The same rule `ServicesPanel.webUrl` uses today, plus the path:
`<protocol>://<host><port><path>`. `host` is `<name>.mesh`, or the record IP
for dotted mDNS-style names. The port is omitted when it is the scheme
default. `path` defaults to `/`. Only `http` and `https` protocols qualify as
mini-apps. As a final check, `new URL(entry).origin` must equal the origin
derived from the record. Path validation is defense in depth, not the only
origin guard.

## Open questions for advise

- Should a card also require the record's owner to be a known fleet node,
  or is that a later trust rung? Proposed: later. The directory already
  exposes only published records.
- 16 KiB message cap and 120–640 px clamp are guesses. Advise may tune them.
