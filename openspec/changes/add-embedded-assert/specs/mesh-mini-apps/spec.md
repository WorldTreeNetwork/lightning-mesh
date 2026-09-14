## ADDED Requirements

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

The pending record SHALL also end, with the sheet closed, nothing signed and
a terminal response only if none was sent, when:
- the visitor denies or dismisses
- the iframe fires `load` again, is removed, or is replaced

While a request is pending, further requests from the same card SHALL be
dropped without a response. A request from any other card SHALL first apply
the deadline check above, which may end an overdue record. It SHALL then
open its own sheet if the slot is free, or else be answered at once on its
own port with error `interaction_required`. A busy-slot rejection SHALL NOT
change the incumbent record or start or extend any card's cooldown.

Cooldown SHALL be keyed by entry origin. When a request ends without
approval (deny, dismiss or expiry), that origin SHALL enter a cooldown.
During it, requests from any card of that origin are answered at once with
`interaction_required` and no sheet opens. The cooldown SHALL be 30 seconds
and SHALL double on each consecutive unapproved ending, up to 10 minutes. It
SHALL reset only when the visitor approves a request from that origin, or on
a direct, trusted visitor gesture (`event.isTrusted`) on that app's open
control in the hello.mesh shelf. Iframe navigation or reload, bridge
traffic, and card teardown or re-creation that isn't caused by that gesture
SHALL NOT reset or shorten it.

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
- THEN B receives error `interaction_required` on its port, A's sheet still names A's origin and is bound to A's nonce, and neither origin's cooldown changes

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
- THEN no sheet opens and it receives `interaction_required`, and after a second unapproved ending the cooldown for that origin is 60 seconds

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
window. On deny or dismiss it SHALL send error `access_denied` on the port.

For `prompt: none` it SHALL sign without a sheet only when a key exists, the
audience is already in the approvals store shared with `/assert`, and the
origin is not in cooldown. It SHALL otherwise answer `interaction_required`
on the port. Every successful `prompt: none` issuance SHALL show "Identity
shared with `<origin>`" in hello.mesh chrome on that card. It SHALL NOT
claim the app signed the visitor in, and the indicator SHALL clear when the
card's iframe loads a new document or is removed. Approving in either
transport SHALL record the audience in that shared store.

#### Scenario: Approve yields a verifiable token

- GIVEN a visitor with a key and a card from `http://keyed.mesh:3000` that sent nonce `n` with a port
- WHEN the visitor approves
- THEN the token arrives on that port, and `verifyAssertion(token, "http://keyed.mesh:3000", n)` accepts it

#### Scenario: Same-origin sibling does not receive the response

- GIVEN two inserted cards, both from `http://keyed.mesh:3000`, and the first sent the pending request
- WHEN the visitor approves
- THEN only the port designated by the first card's request receives `identity.response`

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

## MODIFIED Requirements

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
