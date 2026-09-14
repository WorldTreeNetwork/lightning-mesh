## ADDED Requirements

### Requirement: App marker on service records

A directory service SHALL be treated as a mini-app when, and only when, its
protocol is `http` or `https` and its `txt` carries `app=v1`. An optional
`txt` `path` SHALL name the entry path. It SHALL begin with `/`, SHALL NOT
contain `//`, a scheme, `\`, or control characters, and SHALL default to
`/` when absent. A service whose marker or path is invalid SHALL NOT be a
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

hello.mesh SHALL request `<entry origin>/.well-known/mesh-app.json` from the
browser with a 3-second timeout. A valid manifest SHALL have `v` equal to
`1` and MAY carry `name` (≤ 40 chars), `description` (≤ 140 chars), `icon`
(a same-origin path or a `data:image/` URI ≤ 64 KiB), `embed` (`card` or
`link`, default `link`) and `height` (CSS px, clamped to 120–640). Over-long
strings SHALL be truncated. Manifest values SHALL be rendered only as text
or as an image source, never as markup. When the manifest is missing,
unreachable, blocked by CORS, times out, or is invalid, hello.mesh SHALL
fall back to the service name and `embed: link`.

#### Scenario: Valid manifest

- GIVEN an app whose manifest is `{"v":1,"name":"Keyed","embed":"card","height":900}`
- WHEN hello.mesh loads it
- THEN the app shows as "Keyed", embed mode `card`, height 640

#### Scenario: Manifest blocked by CORS

- GIVEN an app whose manifest response has no `Access-Control-Allow-Origin`
- WHEN hello.mesh loads it
- THEN the app shows under its service name with embed mode `link`

#### Scenario: Markup in manifest is inert

- GIVEN a manifest whose `name` is `<img src=x onerror=alert(1)>`
- WHEN hello.mesh renders the app
- THEN the literal text is shown and no element or script is created

### Requirement: Sandboxed insertion

hello.mesh SHALL insert a mini-app only as an iframe whose `src` is the
entry URL, only after the visitor chooses to open it, and only when the
manifest embed mode is `card` and the entry origin differs from every
hello.mesh origin (the current page origin and `http://hello.mesh`).
The iframe SHALL carry `sandbox="allow-scripts allow-same-origin allow-forms
allow-popups allow-popups-to-escape-sandbox"`, `allow=""`, and
`referrerpolicy="no-referrer"`. hello.mesh SHALL NOT load app-supplied
script, style or markup into its own document. Every inserted card SHALL
keep an open-in-new-tab control. Otherwise the app SHALL be offered as a
link that opens the entry URL in a new tab.

#### Scenario: Card opens on tap

- GIVEN a mini-app with embed mode `card` on origin `http://keyed.mesh:3000`
- WHEN the visitor opens it from hello.mesh
- THEN a sandboxed iframe loads the entry URL and an open-in-new-tab control is visible

#### Scenario: Nothing loads before the tap

- GIVEN three card-mode mini-apps in the directory
- WHEN hello.mesh renders the page
- THEN no request is made to any app entry URL

#### Scenario: Same-origin entry is refused as a card

- GIVEN a mini-app whose entry origin equals the page origin `http://10.42.7.1`
- WHEN hello.mesh decides how to present it
- THEN it is offered as a link, never as an iframe

### Requirement: Bridge envelope

Messages between hello.mesh and an inserted mini-app SHALL be `postMessage`
objects carrying `mesh: "mini-app/v1"` and a `type`. hello.mesh SHALL accept
a message only when its source is that iframe's window and its origin equals
that iframe's entry origin. It SHALL ignore messages with an unknown `mesh`
value or `type`, and messages over 16 KiB serialized. hello.mesh SHALL send
messages only with `targetOrigin` equal to the entry origin. v1 types SHALL
be: `ready` (app to host), `init` with `v: 1` (host to app, once, after the
first `ready`), `resize` with `height` (app to host, clamped to 120–640 CSS
px), and `open` with `url` (app to host; opened in a new tab with
`noopener` only when the scheme is `http` or `https`).

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

#### Scenario: Only web URLs open

- GIVEN an inserted card
- WHEN the app sends `open` with url `javascript:alert(1)`
- THEN nothing opens
