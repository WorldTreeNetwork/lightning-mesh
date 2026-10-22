# mesh-mini-apps

A published service is a mini-app when its TXT carries `app=v1`. hello.mesh
presents those services in an Apps shelf above Services, inserting each
only as a tap-to-load sandboxed iframe on the app's own origin, or as a
link-out tile. App-supplied code never runs in a hello.mesh origin.
Manifests are fetched node-side and served at `GET /api/apps`. An inserted
card may request a `mjolnir-identity-assert` v1 token over the bridge;
consent is drawn by hello.mesh, and the response travels only on a one-shot
MessagePort. Folded from `add-mini-app-contract`, `add-app-shelf`, and
`add-embedded-assert` (2026-09-15), and `add-app-manifest-publish`
(2026-10-21).

Code: `hello-mesh-web/src/lib/miniapp/contract.ts`,
`hello-mesh-web/src/lib/miniapp/apps.ts`,
`hello-mesh-web/src/lib/miniapp/identity-bridge.ts`,
`hello-mesh-web/src/lib/components/AppsPanel.svelte`,
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

Messages from an inserted mini-app to hello.mesh, and window messages from
hello.mesh to it, SHALL be `postMessage` objects carrying
`mesh: "mini-app/v1"` and a `type`. hello.mesh SHALL accept a window message
only when its source is that iframe's window and its origin equals that
iframe's entry origin. It SHALL ignore window messages with an unknown
`mesh` value or `type`, messages that fail `JSON.stringify` (cycles,
`BigInt` and the like; the failure SHALL be caught, not thrown), and
messages whose `JSON.stringify` output exceeds 16 KiB of UTF-8 bytes.
hello.mesh SHALL send window messages only with `targetOrigin` equal to the
entry origin.

v1 types SHALL be:
- `ready` (app to host)
- `init` with `v: 1` (host to app, once, after the first `ready`)
- `resize` with `height` (app to host; `height` SHALL be a finite number or
  the message is ignored, then clamped to 120–640 CSS px by the same function
  that clamps manifest heights)
- `open` with `url` (app to host; opened in a new tab with `noopener` only
  when the scheme is `http` or `https`)
- `identity.request` (app to host, a window message subject to every check
  above, transferring exactly one `MessagePort`)
- `identity.response` (host to app)

`identity.response` is the sole message not sent as a window message. It
SHALL be sent at most once, only on the `MessagePort` transferred by the
authenticated `identity.request` it answers. That port is then closed. Every
identity token and identity error SHALL travel this way, never as a window
message. hello.mesh SHALL NOT receive or interpret messages on that port.

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

#### Scenario: Identity request from a foreign origin is ignored

- GIVEN a card inserted from `http://keyed.mesh:3000`
- WHEN an `identity.request` with a port arrives with origin `http://other.mesh`
- THEN no pending record is created and nothing is sent on that port

#### Scenario: Identity never travels as a window message

- GIVEN any identity request outcome (token, `access_denied`, `interaction_required`, `invalid_request`)
- WHEN hello.mesh responds
- THEN the response is sent only on the request's transferred port and no window message carries it

### Requirement: Apps shelf on hello.mesh

hello.mesh SHALL present mini-apps in an Apps shelf on the front desk,
above the Services list. The shelf SHALL read `GET /api/apps` and the
directory services already polled for the page. A service SHALL appear as
an app tile only when `isMiniApp` is true. Tile name, description, icon
and embed mode SHALL come from the `/api/apps` record when present, else
from the service name and `embed: link`. Manifest strings SHALL render as
text (or `<img>` for an inlined icon), never as markup.

A card-mode tile that `canEmbed` SHALL create no iframe until the visitor
opens it. After open, insertion SHALL follow Sandboxed insertion, using
`pageOrigin = window.location.origin` (never the literal `hello.mesh`).
Iframe `src` and link `href` SHALL come only from `entryUrl(service)`.
Every tile SHALL keep an open-in-new-tab control
(`target="_blank" rel="noopener noreferrer"`) and SHALL show the entry
host as text beside the display name. A record with `manifest: null`
(missing, expired or invalid) SHALL show a link-out tile. A record with
`stale: true` and a kept manifest SHALL honour that manifest. Closing a
card SHALL remove the iframe element and its `message` listener. At most
one card SHALL be open at a time. Mini-apps SHALL NOT also appear in
Services. An empty Apps shelf SHALL be a compact line, not a full empty
card. `/api/apps` refresh SHALL ride the directory poll (5 s) or slower.
The page SHALL NOT add `<link rel="preconnect|prefetch|dns-prefetch|icon">`
to app entry origins. Apps SHALL NOT contact app hosts from the browser
except the entry URL of an opened card.

The tile set SHALL be the directory filtered by `isMiniApp`; `/api/apps`
is a per-service lookup. If that request fails or is not yet loaded, tiles
SHALL still render as link-out.

#### Scenario: Card tile opens on tap

- GIVEN `/api/apps` has a valid card-mode mini-app whose entry origin is not key-bearing
- WHEN the visitor opens its tile
- THEN a sandboxed iframe loads the entry URL and an open-in-new-tab control is visible

#### Scenario: Failed manifest is still a tile

- GIVEN a mini-app whose `/api/apps` record has no manifest
- WHEN Apps renders
- THEN a link-out tile using the service name is shown and no iframe is created

#### Scenario: Page load does not fetch app hosts

- GIVEN three mini-apps in the directory
- WHEN a visitor loads hello.mesh
- THEN the page's network requests go only to a hello.mesh origin until a card is opened

#### Scenario: Unmarked services stay in Services

- GIVEN a web service with no `app=v1`
- WHEN hello.mesh renders
- THEN it appears in Services and not in Apps

### Requirement: Identity request over the bridge

hello.mesh SHALL accept an `identity.request` bridge message only from a
registered inserted card, meaning `event.source` is that card's iframe window
and `event.origin` is its entry origin. A `null` origin is never accepted.
The message SHALL carry `nonce` (even-length hex, 16–128 chars) and `prompt`
(`consent` or `none`, default `consent`). It SHALL also transfer exactly
one `MessagePort`. All app-to-host identity input (nonce, prompt) SHALL come
only from that authenticated window event. The assertion audience SHALL be
the card's entry origin, and the message SHALL NOT be able to supply a
different audience. Audience and nonce SHALL be fixed when the request is
accepted and SHALL NOT change afterwards. A request without exactly one port
SHALL be dropped without a response. A malformed request that carries a port
SHALL be answered on that port with error `invalid_request`, echoing the
nonce only when it is well formed.

The transferred port is a **one-shot response capability designated by the
authenticated request**. hello.mesh SHALL NOT verify, and SHALL NOT claim,
which document holds the port's other end. A document that later replaces
the requester in the iframe does not inherit that endpoint. A requester that
deliberately hands its endpoint to another context has only delegated a
token it could disclose itself. hello.mesh SHALL treat the port as strictly
one-way. It SHALL NOT call `start()` on it, attach a `message` listener, or
interpret any message arriving on it. It SHALL send at most one terminal
`identity.response` on it and then close it.

hello.mesh SHALL hold at most one pending identity request across all cards.
The pending record SHALL capture the card, its entry origin, the nonce, the
port, and an absolute deadline equal to the acceptance time plus 60 seconds
on the host's monotonic clock. Handling of every identity request and every
consent decision SHALL first compare the current time with any pending
record's deadline. At or after the deadline, expiry wins: the sheet closes,
the record's port receives `interaction_required`, nothing is signed, and
the record ends. Before the deadline, Allow SHALL atomically claim the record
before signing. A claimed record SHALL NOT expire, be claimed again, or be
answered twice. A timer MAY close the sheet at the deadline, but correctness
SHALL NOT depend on that timer running on time.

A request is **response-eligible** when all of these hold:
- it passed the window checks (`source`, `origin`, envelope, size)
- it transferred exactly one port
- it isn't a duplicate from an app whose request is already pending

A request that isn't response-eligible SHALL be dropped. It gets no
response, no pending record, and no change to any cooldown. Every
response-eligible request SHALL end with exactly one outcome, sent once on
its port before the port is closed:

| Ending | Error sent | Starts cooldown |
|---|---|---|
| Visitor approves (or `prompt: none` signs) | none, a token | no, resets it |
| Visitor presses Deny | `access_denied` | yes |
| Visitor dismisses the sheet without choosing (no key or key) | `interaction_required` | yes |
| Deadline reached | `interaction_required` | yes |
| The app's iframe fires `load` again (app-driven navigation or reload) | `interaction_required` | yes |
| Visitor closes the card, or the card is removed by the host | `interaction_required` | no |
| The app's record is republished with a different entry origin while its card is open (the host closes the card) | `interaction_required` | yes |
| Slot busy with another app's request | `interaction_required` | no |
| App in cooldown | `interaction_required` | no (unchanged) |
| `prompt: none` without key or prior approval | `interaction_required` | no |
| Malformed request with a port | `invalid_request` | no |

The error goes to the port even when the app's document has gone. It
carries no identity, and closing the port releases it.

While a request is pending, further requests from the same card SHALL be
dropped without a response. A request from any other card SHALL first apply
the deadline check above, which may end an overdue record. It SHALL then
open its own sheet if the slot is free, or else be answered at once on its
own port with error `interaction_required`. A busy-slot rejection SHALL NOT
change the incumbent record or start or extend any app's cooldown.

Cooldown SHALL be keyed by the **service name**, the stable `ServiceBookV2`
key under which the shelf presents one app. It is not keyed by card
instance, which the host re-creates. It is not keyed by the record's
mutable ip, port or protocol, which the publisher can republish. And it is
not keyed by bare origin. A republish of the same name SHALL keep that
name's cooldown state. Distinct service names SHALL never share cooldown
state. When a request ends in an outcome marked "starts cooldown" above,
that app SHALL enter a cooldown. During it, its requests are answered at
once with `interaction_required` and no sheet opens. The cooldown SHALL be
30 seconds and SHALL double on each consecutive cooldown-starting ending, up
to 10 minutes. It SHALL reset only when the visitor approves a request from
that app, or on a direct, trusted visitor gesture (`event.isTrusted`) on that
app's open control in the hello.mesh shelf. Iframe navigation or reload,
bridge traffic, and card teardown or re-creation that isn't caused by that
gesture SHALL NOT reset or shorten it. One app's cooldown SHALL NOT affect
any other app.

#### Scenario: Audience comes from the origin

- GIVEN a card inserted from `http://keyed.mesh:3000`
- WHEN it sends `identity.request` with a port and an extra field `audience: "http://bank.mesh"`
- THEN any assertion issued has audience `http://keyed.mesh:3000`

#### Scenario: Malformed nonce

- GIVEN an inserted card
- WHEN it sends `identity.request` with a port and nonce `xyz`
- THEN it receives `identity.response` on that port with error `invalid_request` and no nonce

#### Scenario: Request without a port

- GIVEN an inserted card
- WHEN it sends `identity.request` with no transferred port
- THEN no sheet opens and no response is sent anywhere

#### Scenario: Messages on the response port are ignored

- GIVEN a pending request from `http://keyed.mesh:3000` with nonce `n`
- WHEN the app posts messages on the transferred port claiming nonce `m`, audience `http://bank.mesh`, an approval, or a second request
- THEN the pending record, its audience and nonce are unchanged, no sheet state changes, and after the visitor approves exactly one response is sent, carrying nonce `n`

#### Scenario: Duplicate request from the same card

- GIVEN a pending request from a card
- WHEN that card sends another `identity.request`
- THEN no second sheet opens and no response is sent for the second request

#### Scenario: Two cards request at once

- GIVEN cards A and B are both inserted and A's request is pending and not overdue
- WHEN B sends `identity.request`
- THEN B receives error `interaction_required` on its port, A's sheet still names A's origin and is bound to A's nonce, and neither app's cooldown changes

#### Scenario: Hostile card cannot hold the slot

- GIVEN card A opened a request and the visitor walks away
- WHEN 60 seconds pass and card B then sends `identity.request`
- THEN A's sheet has closed, A's port received `interaction_required`, and B's sheet opens

#### Scenario: Overdue record is ended even if its timer is late

- GIVEN A's request is 61 seconds old and its expiry timer has not yet run
- WHEN card B sends `identity.request`, or the visitor clicks Allow on A's sheet
- THEN A's record ends by expiry first, A's port receives `interaction_required`, nothing is signed for A, and B's request is handled against a free slot

#### Scenario: Allow just before the deadline wins

- GIVEN A's request is 59.9 seconds old
- WHEN the visitor clicks Allow and the expiry timer fires during signing
- THEN the record was claimed before signing, exactly one `identity.response` carrying the token is sent, and no `interaction_required` follows

#### Scenario: Denial loop is throttled

- GIVEN the visitor denied a request from `http://keyed.mesh:3000`
- WHEN that app sends `identity.request` again 5 seconds later
- THEN no sheet opens and it receives `interaction_required`, and after a second cooldown-starting ending that app's cooldown is 60 seconds

#### Scenario: One app's cooldown does not throttle another

- GIVEN apps Keyed (`http://keyed.mesh:3000`) and Guestbook (`http://guestbook.mesh`) are both inserted, and Keyed is in cooldown
- WHEN Guestbook sends `identity.request`
- THEN Guestbook's sheet opens normally

#### Scenario: Dismiss versus Deny

- GIVEN a visitor with a key and a pending consent sheet
- WHEN the visitor presses Deny, or in a second run closes the sheet without choosing
- THEN Deny sends `access_denied` and dismissal sends `interaction_required`, and both start the app's cooldown

#### Scenario: App reload ends the request with one outcome

- GIVEN a pending request from Keyed
- WHEN Keyed's iframe reloads itself before the visitor decides
- THEN the sheet closes, the designated port receives exactly one `interaction_required`, nothing is signed, and Keyed's cooldown starts

#### Scenario: App cannot reset its cooldown

- GIVEN `http://keyed.mesh:3000` is in cooldown
- WHEN its iframe reloads itself, posts `ready` again, or navigates away and back
- THEN the cooldown is unchanged, and it resets only after the visitor directly taps that app's open control in the shelf

#### Scenario: Navigate away and back invalidates the request

- GIVEN a pending request from a card at `http://keyed.mesh:3000`
- WHEN the iframe navigates to `http://other.mesh` and then back to `http://keyed.mesh:3000` before the visitor approves
- THEN the sheet closes, nothing is signed, and no token is sent

### Requirement: Consent rendered by the host

For `prompt: consent`, hello.mesh SHALL render consent in its own document,
outside the iframe. The consent SHALL show the entry origin as the primary
label, the manifest name only as a label for what the app calls itself, the
visitor's display name, and the first 8 hex chars of the pubkey. The Allow
action SHALL sign only for the pending record the sheet displays, and only
after atomically claiming it under the deadline rule above. When the visitor
has no key, consent SHALL offer to create one first.

When its own window is framed (`window.top !== window`), by any page
**including a hello.mesh origin**, hello.mesh SHALL fail closed before
loading identity, reading approvals, registering any bridge listener, or
calling any signing function. This applies to both the bridge and `/assert`,
including a previously approved `prompt=none`. This runtime check is the
only defence against same-origin framing, since CSP `'self'` allows it. The
SPA and static hello.mesh application HTML responses SHALL carry
`Content-Security-Policy: frame-ancestors 'self'`. The captive-portal page
(`PORTAL_HTML`) and the byte-exact probe success bodies SHALL NOT carry it
and SHALL NOT otherwise change.

#### Scenario: Consent names the origin first

- GIVEN a card from `http://keyed.mesh:3000` whose manifest name is "Bank of Mesh"
- WHEN it requests identity with `prompt: consent`
- THEN the sheet leads with `http://keyed.mesh:3000` and shows "Bank of Mesh" as what the app calls itself

#### Scenario: Foreign framing cannot sign silently

- GIVEN a visitor who earlier approved `http://evil.mesh` through `/assert`
- WHEN `http://evil.mesh` frames `http://hello.mesh/assert?audience=http://evil.mesh&…&prompt=none`, and the frame renders despite CSP
- THEN no identity or approval is read, nothing is signed, and no redirect carries a token

#### Scenario: Same-origin self-framing cannot sign

- GIVEN a visitor who earlier approved `http://keyed.mesh:3000`
- WHEN a `http://hello.mesh` page frames `http://hello.mesh/assert?…&prompt=none` for that audience, or frames the hello.mesh front desk and an embedded card inside it sends `identity.request` with `prompt: none`
- THEN in both cases the framed hello.mesh reads no identity or approvals, registers no bridge listener, and signs nothing

#### Scenario: hello.mesh application HTML refuses foreign framing

- GIVEN any SPA or static hello.mesh HTML response
- WHEN its headers are inspected
- THEN it carries `Content-Security-Policy: frame-ancestors 'self'`

#### Scenario: Captive portal is untouched

- GIVEN a client whose OS probe is redirected to the node
- WHEN it fetches the portal page, passes through, and re-probes
- THEN the portal page has no `frame-ancestors` header and the probe success body is byte-identical to the OS payload

#### Scenario: No key yet

- GIVEN a visitor with no IdentiKey
- WHEN a card requests identity with `prompt: consent`
- THEN the sheet offers to create an identity, and dismissing it sends error `interaction_required` on the port

### Requirement: Identity response over the bridge

On approve, hello.mesh SHALL send `identity.response`, carrying the fixed
nonce and a token built exactly as `mjolnir-identity-assert` v1 (same
payload key order, domain prefix, 300-second expiry, base64url encoding),
only on the pending record's port, and then close that port. It SHALL NOT
send an identity token or identity error through `window.postMessage` to any
window. Every non-token ending SHALL send the single error given by the
outcome table in "Identity request over the bridge".

For `prompt: none` it SHALL sign without a sheet only when a key exists, the
audience is already in the approvals store shared with `/assert`, and the
app is not in cooldown. It SHALL otherwise answer `interaction_required`
on the port. Every successful `prompt: none` issuance SHALL show "Identity
shared with `<origin>`" in hello.mesh chrome on that card. It SHALL NOT
claim the app signed the visitor in, and the indicator SHALL clear when the
card's iframe loads a new document or is removed. Approving in either
transport SHALL record the audience in that shared store.

#### Scenario: Approve yields a verifiable token

- GIVEN a visitor with a key and a card from `http://keyed.mesh:3000` that sent nonce `n` with a port
- WHEN the visitor approves
- THEN the token arrives on that port, and `verifyAssertion(token, "http://keyed.mesh:3000", n)` accepts it

#### Scenario: Re-opened card does not receive the old response

- GIVEN a pending request from Keyed's card
- WHEN the visitor closes that card and re-opens Keyed from the shelf, creating a new iframe, then the new card sends its own request
- THEN the first request already ended with one `interaction_required` on its port, no token is ever sent on it, and the new request is handled on its own port

#### Scenario: Replacement document does not inherit the response port

- GIVEN a pending request whose port was designated by the card's current document, which did not hand its endpoint to anyone
- WHEN the iframe navigates to a new `http://keyed.mesh:3000` document that is already listening for window messages, and the visitor clicks Allow before the parent sees `load`
- THEN the new document receives no token

#### Scenario: Silent sharing after earlier approval is disclosed

- GIVEN `http://keyed.mesh:3000` was approved earlier through `/assert`
- WHEN the card sends `identity.request` with `prompt: none` and a port
- THEN a token arrives on the port with no sheet shown, and the card header in hello.mesh reads "Identity shared with http://keyed.mesh:3000" until the card loads a new document

#### Scenario: Silent request without approval

- GIVEN an audience that was never approved
- WHEN the card sends `identity.request` with `prompt: none`
- THEN it receives error `interaction_required` on the port and no sheet is shown

#### Scenario: Response never goes to a navigated frame

- GIVEN a card that navigated to `http://other.mesh` after requesting identity
- WHEN the visitor approves
- THEN nothing is signed and no token is sent

### Requirement: One card per app

hello.mesh SHALL keep at most one inserted card per service name at a time.
Opening an app that already has a card SHALL focus that card rather than
insert a second. When the directory record for an open card's name changes
so that its entry origin changes (a new protocol or port), hello.mesh SHALL
close that card, ending any pending request as in the outcome table. When
only the record's ip changes, the entry origin is unchanged, so the card
stays. Because a non-reserved `.mesh` name host maps to exactly one service
name, and IP-literal hosts are never embedded, two inserted cards never
share an entry origin.

#### Scenario: Cooldown survives a port republish

- GIVEN Keyed (`keyed`, port 3000) is in a 60-second cooldown
- WHEN its publisher republishes `keyed` on port 3001 and the visitor's directory updates
- THEN the open Keyed card (if any) closes, a card opened for the new record is still in the same cooldown, and its requests get `interaction_required` until the cooldown ends

#### Scenario: Cooldown survives an ip republish

- GIVEN Keyed is in cooldown with its card open
- WHEN its publisher republishes `keyed` with a different ip and the same port and protocol
- THEN the card stays open and Keyed remains in the same cooldown

#### Scenario: Different names never share cooldown

- GIVEN `keyed` is in cooldown and `guestbook` is published at the same ip and port as `keyed`'s previous record
- WHEN Guestbook's card sends `identity.request`
- THEN Guestbook's sheet opens normally

#### Scenario: Ineligible requests are dropped

- GIVEN an inserted card
- WHEN it sends `identity.request` without a port, with two ports, or as a duplicate while its app's request is pending
- THEN no response is sent anywhere, no pending record is created or changed, and no cooldown changes

#### Scenario: Opening an open app focuses it

- GIVEN Keyed's card is inserted
- WHEN the visitor taps Keyed's open control again
- THEN no second iframe is created, the existing card is focused, and Keyed's cooldown (if any) resets per the trusted-gesture rule

### Requirement: App marker accepted on every publish path

The system SHALL accept the mini-app marker (version `1` and an optional entry
path) on every path that publishes a name. That covers key-owned name claims
(`POST /api/name-claim`, as an optional `app` object outside the signed message)
and the node control API (`POST /v0/publish` and the `mjolnir-meshd publish`
command). On the control API the marker is present if and only if the TXT
key `app` is present. When `app` is absent, `path` and every other TXT key
SHALL be opaque and untouched (plain services may publish `--txt path=print`
today). When `app` is present it SHALL be `v1` byte-exact, and `path`, if
present, SHALL pass the shared rule set. All paths SHALL validate the marker
with one shared rule set, the same as the living `App marker on service records`
requirement and `contract.ts` `appPath`:
- version equals `1`
- the path begins with `/`
- the path has no `//`, `\`, scheme, or control characters
- an omitted path is `/`

There is no separate publish-time length ceiling. An invalid marker SHALL be
rejected with an error before anything is spooled or published, and SHALL NOT
alter an existing lease or service. A published marker SHALL appear in the
directory as TXT `app=v1` and, unless the path is `/`, `path=<path>`.
`{"app":{"v":1}}` SHALL be accepted and SHALL project like path `/`.
`--app-path` SHALL be rejected unless `--app` is also set. Consumers SHALL
still re-validate the marker.

#### Scenario: Self-serve mini-app without SSH

- GIVEN an app holding a valid key-owned lease on `walkie-talkie`
- WHEN it renews with a valid signature and `"app":{"v":1,"path":"/"}`
- THEN within one sweep the directory lists `walkie-talkie` with TXT `app=v1` and the name appears in `/api/apps`

#### Scenario: Invalid path on a name claim

- GIVEN an app holding a valid lease without an app marker
- WHEN it submits a validly signed claim with `"app":{"v":1,"path":"//evil.example"}`
- THEN the request is rejected with `400`, nothing is spooled, and the existing lease is unchanged

#### Scenario: Unsupported version on the control API

- GIVEN an operator on a router
- WHEN they run `mjolnir-meshd publish foo --port 80 --txt app=v2`
- THEN the publish is rejected with a reason naming the unsupported version and no service record changes

#### Scenario: Marker omitted on renewal

- GIVEN a lease whose last claim carried an app marker
- WHEN the owner renews without an `app` field
- THEN the directory entry no longer carries TXT `app` or `path`

#### Scenario: Omitted path is slash

- GIVEN an app holding a valid key-owned lease
- WHEN it claims with `"app":{"v":1}` and no `path` field
- THEN the request is accepted and the directory lists TXT `app=v1` without `path=`

#### Scenario: Plain service with path TXT and no app still publishes

- GIVEN an operator on a router
- WHEN they run `mjolnir-meshd publish printer --port 631 --txt path=print` with no `app` TXT key
- THEN the publish succeeds and the directory lists `printer` with TXT `path=print` and no `app`
- AND the service is not a mini-app

#### Scenario: --app-path requires --app

- GIVEN an operator on a router
- WHEN they run `mjolnir-meshd publish foo --port 80 --app-path /x` without `--app`
- THEN the command is rejected and no service record changes

#### Scenario: Signed ceremony unchanged

- GIVEN a client that signs `mjolnir-name-claim:v1\n<challenge>\n<name>\n<port>` exactly as before
- WHEN it adds or omits the `app` field
- THEN the same signature verifies in both cases

