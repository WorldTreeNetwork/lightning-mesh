# mesh-mini-apps

A published service is a mini-app when its TXT carries `app=v1`. hello.mesh
inserts it only as a tap-to-load sandboxed iframe on the app's own origin,
or as a link-out tile. App-supplied code never runs in a hello.mesh origin.
Manifests are fetched node-side and served at `GET /api/apps`. Folded from
`add-mini-app-contract` (2026-09-15).

Code: `hello-mesh-web/src/lib/miniapp/contract.ts`,
`crates/mjolnir-hello/src/apps.rs` (`GET /api/apps`).

## Requirements

### Requirement: App marker on service records

A directory service SHALL be treated as a mini-app when, and only when, its
protocol is `http` or `https` and its `txt` carries `app=v1`. An optional
`txt` `path` SHALL name the entry path. It SHALL begin with `/`, SHALL NOT
contain `//`, a scheme, `\`, or control characters, and SHALL default to
`/` when absent. After the entry URL is built, its origin SHALL equal the
origin derived from the record (protocol, host, port) or the service SHALL
NOT be a mini-app. A service whose marker or path is invalid SHALL NOT be a
mini-app and SHALL still be listed as an ordinary service.

#### Scenario: Marked web service is a mini-app

- GIVEN a service `keyed` with protocol `http`, port `3000`, txt `app=v1`, `path=/app`
- WHEN hello.mesh evaluates the directory
- THEN it is a mini-app whose entry URL is `http://keyed.mesh:3000/app`

#### Scenario: Unmarked service stays a plain service

- GIVEN a service with protocol `http` and no `app` txt key
- WHEN hello.mesh evaluates the directory
- THEN it is not a mini-app and still appears in the Services panel

#### Scenario: Bad path is not an app

- GIVEN a service with txt `app=v1` and `path=//evil.example/x`
- WHEN hello.mesh evaluates the directory
- THEN it is not a mini-app and still appears in the Services panel

#### Scenario: Non-web protocol is not an app

- GIVEN a service with protocol `ipp` and txt `app=v1`
- WHEN hello.mesh evaluates the directory
- THEN it is not a mini-app

### Requirement: App manifest

Each node's hello service SHALL fetch `/.well-known/mesh-app.json` for every
mini-app in its directory **server-side**, and SHALL serve the validated
results at `GET /api/apps`. The visitor's browser SHALL NOT contact any app
host to discover or decorate apps.

The fetch SHALL go only to the record's own `ip` and `port`, and only when
that `ip` is inside the mesh client or overlay ranges (`10.42.0.0/16`,
`10.254.0.0/16`). It SHALL send `Host` set to the entry host and use `GET`
on that one path. It SHALL NOT follow redirects, SHALL time out after 3
seconds, and SHALL cap the body at 128 KiB. It SHALL re-fetch at most once
every 5 minutes per app and SHALL keep the last good manifest for up to 1
hour. For `https` entries the fetch MAY accept an unverified certificate,
because the manifest is presentation data only.

A valid manifest SHALL have `v` equal to `1` and MAY carry `name` (≤ 40
chars), `description` (≤ 140 chars), `icon` (a same-origin path, which the
node fetches under the same rules and inlines as a `data:image/` URI ≤ 64
KiB of type png, jpeg, webp or svg+xml), `embed` (`card` or `link`, default
`link`) and `height` (CSS px, clamped to 120–640). Over-long strings SHALL
be truncated. The node SHALL validate, and the browser SHALL re-validate
`/api/apps` entries with the same rules. Manifest values SHALL be rendered
only as text or as an `<img>` source, never as markup. When the manifest is
missing, unreachable, out of range, redirected, over size, timed out or
invalid, the app SHALL fall back to its service name and `embed: link`.

#### Scenario: Valid manifest

- GIVEN an app whose manifest is `{"v":1,"name":"Keyed","embed":"card","height":900}`
- WHEN the node fetches it and hello.mesh reads `/api/apps`
- THEN the app shows as "Keyed", embed mode `card`, height 640

#### Scenario: App without CORS is still decorated

- GIVEN an app whose manifest response has no `Access-Control-Allow-Origin`
- WHEN the node fetches it
- THEN `/api/apps` carries its name, icon and embed mode

#### Scenario: Self-signed https app is decorated

- GIVEN an `https` app on `10.42.12.165:443` with a self-signed certificate and a valid manifest
- WHEN the node fetches it
- THEN `/api/apps` carries its manifest fields

#### Scenario: Out-of-range address is not fetched

- GIVEN a mini-app record whose `ip` is `203.0.113.9`
- WHEN the node refreshes manifests
- THEN no request is made to that address and the app falls back to `embed: link`

#### Scenario: Redirect is not followed

- GIVEN an app whose manifest path answers `302` to another host
- WHEN the node fetches it
- THEN the redirect is not followed and the app falls back to `embed: link`

#### Scenario: Visitor's browser contacts no app host on load

- GIVEN a directory with three mini-apps
- WHEN a visitor loads hello.mesh
- THEN every request the page makes goes to a hello.mesh origin

#### Scenario: Markup in manifest is inert

- GIVEN a manifest whose `name` is `<img src=x onerror=alert(1)>`
- WHEN hello.mesh renders the app
- THEN the literal text is shown and no element or script is created

### Requirement: Sandboxed insertion

hello.mesh SHALL insert a mini-app only as an iframe whose `src` is the
entry URL, only after the visitor chooses to open it, and only when the
manifest embed mode is `card` and the entry origin is not a key-bearing
hello origin. An entry origin SHALL be treated as key-bearing, and so
never embedded, when any of these holds: its host is an IP literal (every
node's hello.mesh is also served at its LAN gateway IP, and each such
origin can hold a key); its host is a reserved well-known name
(`hello.mesh`, `id.mesh`, and any future name that serves the hello.mesh
page); or it equals the current page origin. The iframe SHALL carry
`sandbox="allow-scripts allow-same-origin allow-forms allow-popups
allow-popups-to-escape-sandbox"`, `allow=""`, and
`referrerpolicy="no-referrer"`. hello.mesh SHALL NOT load app-supplied
script, style or markup into its own document. Every inserted card SHALL
keep an open-in-new-tab control. Otherwise the app SHALL be offered as a
link that opens the entry URL in a new tab.

#### Scenario: Card opens on tap

- GIVEN a mini-app with embed mode `card` on origin `http://keyed.mesh:3000`
- WHEN the visitor opens it from hello.mesh
- THEN a sandboxed iframe loads the entry URL and an open-in-new-tab control is visible

#### Scenario: No frame loads before the tap

- GIVEN three card-mode mini-apps in the directory
- WHEN hello.mesh renders the page
- THEN no iframe is created and no request is made to any app entry URL

#### Scenario: Same-origin entry is refused as a card

- GIVEN a mini-app whose entry origin equals the page origin `http://10.42.7.1`
- WHEN hello.mesh decides how to present it
- THEN it is offered as a link, never as an iframe

#### Scenario: Another node's gateway origin is refused as a card

- GIVEN the page is `http://hello.mesh`, the visitor earlier stored a key under `http://10.42.7.1`, and a card-mode mini-app record with a dotted mDNS-style name resolves its entry to `http://10.42.7.1/`
- WHEN hello.mesh decides how to present it
- THEN it is offered as a link, never as an iframe

#### Scenario: Reserved name is refused as a card

- GIVEN a card-mode mini-app whose entry origin is `http://id.mesh`
- WHEN hello.mesh decides how to present it
- THEN it is offered as a link, never as an iframe

### Requirement: Bridge envelope

Messages between hello.mesh and an inserted mini-app SHALL be `postMessage`
objects carrying `mesh: "mini-app/v1"` and a `type`. hello.mesh SHALL accept
a message only when its source is that iframe's window and its origin equals
that iframe's entry origin. It SHALL ignore messages with an unknown `mesh`
value or `type`, messages that fail `JSON.stringify` (cycles, `BigInt` and
the like; the failure SHALL be caught, not thrown), and messages whose
`JSON.stringify` output exceeds 16 KiB of UTF-8 bytes. hello.mesh SHALL
send messages only with `targetOrigin` equal to the entry origin. v1 types
SHALL be: `ready` (app to host), `init` with `v: 1` (host to app, once,
after the first `ready`), `resize` with `height` (app to host; `height`
SHALL be a finite number or the message is ignored, then clamped to
120–640 CSS px by the same function that clamps manifest heights), and
`open` with `url` (app to host; opened in a new tab with `noopener` only
when the scheme is `http` or `https`).

#### Scenario: Message from a foreign origin is ignored

- GIVEN a card inserted from `http://keyed.mesh:3000`
- WHEN a `resize` message arrives with origin `http://other.mesh`
- THEN the card height does not change

#### Scenario: Frame navigated away gets nothing

- GIVEN a card whose iframe has navigated to `http://other.mesh`
- WHEN hello.mesh sends `init`
- THEN the message is addressed to `http://keyed.mesh:3000` and the browser does not deliver it

#### Scenario: Resize is clamped

- GIVEN an inserted card
- WHEN the app sends `resize` with height `5000`
- THEN the card height becomes 640

#### Scenario: Non-finite height is ignored

- GIVEN an inserted card at height 320
- WHEN the app sends `resize` with height `NaN`, `Infinity`, or `"400"`
- THEN the card height stays 320

#### Scenario: Oversize or unserializable message is ignored

- GIVEN an inserted card
- WHEN the app sends a message whose JSON is 16 KiB + 1 byte of UTF-8, or one containing a `BigInt`
- THEN the message is ignored and no error escapes the listener

#### Scenario: Only web URLs open

- GIVEN an inserted card
- WHEN the app sends `open` with url `javascript:alert(1)`
- THEN nothing opens
