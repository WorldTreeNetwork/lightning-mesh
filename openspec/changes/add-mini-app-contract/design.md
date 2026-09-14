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
`canEmbed` refuses any entry origin equal to a hello.mesh origin: with a
same-origin src, `allow-scripts` + `allow-same-origin` lets the frame reach
the parent and remove its own sandbox. No `allow-top-navigation`; the app
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

Costs:
- The app must answer that GET with `Access-Control-Allow-Origin: *` or
  hello.mesh cannot read it.
- An `https` app with a self-signed certificate (for example today's
  walkie-talkie) fails the fetch and the frame load from an `http`
  hello.mesh page. It degrades to a link tile, where the visitor can accept
  the certificate in a full tab. Stage 1 reference apps should serve `http`
  or a trusted certificate.

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
- Drop messages whose serialized size is over 16 KiB, and any unknown `type`.
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
mini-apps.

## Open questions for advise

- Should a card also require the record's owner to be a known fleet node,
  or is that a later trust rung? Proposed: later. The directory already
  exposes only published records.
- 16 KiB message cap and 120–640 px clamp are guesses. Advise may tune them.
