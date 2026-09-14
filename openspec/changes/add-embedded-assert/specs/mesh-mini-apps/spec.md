## ADDED Requirements

### Requirement: Identity request over the bridge

hello.mesh SHALL accept an `identity.request` bridge message only from a
registered inserted card, meaning `event.source` is that card's iframe window
and `event.origin` is its entry origin. A `null` origin is never accepted.
The message SHALL carry `nonce` (even-length hex, 16–128 chars) and `prompt`
(`consent` or `none`, default `consent`). The assertion audience SHALL be
the card's entry origin, and the message SHALL NOT be able to supply a
different audience. A malformed request SHALL be answered with
`identity.response` error `invalid_request`, echoing the nonce only when it
is well formed.

hello.mesh SHALL hold at most one pending identity request across all cards.
The pending record SHALL capture the card, its iframe window, its entry
origin and the nonce. It SHALL be invalidated, with the sheet closed and no
signing, when that iframe fires `load` again, is removed, or is replaced.
While a request is pending, further requests from the same card SHALL be
dropped without a response. Requests from any other card SHALL be answered
at once with error `interaction_required`.

#### Scenario: Audience comes from the origin

- GIVEN a card inserted from `http://keyed.mesh:3000`
- WHEN it sends `identity.request` with an extra field `audience: "http://bank.mesh"`
- THEN any assertion issued has audience `http://keyed.mesh:3000`

#### Scenario: Malformed nonce

- GIVEN an inserted card
- WHEN it sends `identity.request` with nonce `xyz`
- THEN it receives `identity.response` with error `invalid_request` and no nonce

#### Scenario: Duplicate request from the same card

- GIVEN a pending request from a card
- WHEN that card sends another `identity.request`
- THEN no second sheet opens and no response is sent for the second request

#### Scenario: Two cards request at once

- GIVEN cards A and B are both inserted and A's request is pending
- WHEN B sends `identity.request`
- THEN B receives error `interaction_required`, and the open sheet still names A's origin and is bound to A's nonce

#### Scenario: Navigate away and back invalidates the request

- GIVEN a pending request from a card at `http://keyed.mesh:3000`
- WHEN the iframe navigates to `http://other.mesh` and then back to `http://keyed.mesh:3000` before the visitor approves
- THEN the sheet closes, nothing is signed, and no response is posted

### Requirement: Consent rendered by the host

For `prompt: consent`, hello.mesh SHALL render consent in its own document,
outside the iframe. The consent SHALL show the entry origin as the primary
label, the manifest name only as a label for what the app calls itself, the
visitor's display name, and the first 8 hex chars of the pubkey. The Allow
action SHALL sign only for the pending record the sheet displays (card,
origin, nonce). When the visitor has no key, consent SHALL offer to create
one first.

When its own window is framed (`window.top !== window`), hello.mesh SHALL
fail closed before loading identity, reading approvals, registering any
bridge listener, or calling any signing function. This applies to both the
bridge and `/assert`, including a previously approved `prompt=none`. The
SPA and static hello.mesh application HTML responses SHALL carry
`Content-Security-Policy: frame-ancestors 'self'`. The captive-portal page
(`PORTAL_HTML`) and the byte-exact probe success bodies SHALL NOT carry it
and SHALL NOT otherwise change.

#### Scenario: Consent names the origin first

- GIVEN a card from `http://keyed.mesh:3000` whose manifest name is "Bank of Mesh"
- WHEN it requests identity with `prompt: consent`
- THEN the sheet leads with `http://keyed.mesh:3000` and shows "Bank of Mesh" as what the app calls itself

#### Scenario: Framed hello.mesh cannot sign silently

- GIVEN a visitor who earlier approved `http://evil.mesh` through `/assert`
- WHEN `http://evil.mesh` frames `http://hello.mesh/assert?audience=http://evil.mesh&…&prompt=none`, and the frame renders despite CSP
- THEN no identity or approval is read, nothing is signed, and no redirect carries a token

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
- THEN the sheet offers to create an identity, and dismissing it sends error `interaction_required`

### Requirement: Identity response over the bridge

On approve, hello.mesh SHALL post `identity.response` carrying the echoed
nonce and a token built exactly as `mjolnir-identity-assert` v1 (same
payload key order, domain prefix, 300-second expiry, base64url encoding).
It SHALL post to the iframe window captured in the pending record, with
`targetOrigin` equal to the entry origin, and only while that record is
still valid. On deny it SHALL post error `access_denied`. For
`prompt: none` it SHALL sign without a sheet only when a key exists and the
audience is already in the approvals store shared with `/assert`, and SHALL
otherwise post error `interaction_required`. Every successful `prompt: none`
issuance SHALL show "Identity shared with `<origin>`" in hello.mesh chrome
on that card. It SHALL NOT claim the app signed the visitor in. Approving in
either transport SHALL record the audience in that shared store.

#### Scenario: Approve yields a verifiable token

- GIVEN a visitor with a key and a card from `http://keyed.mesh:3000` that sent nonce `n`
- WHEN the visitor approves
- THEN the card receives a token that `verifyAssertion(token, "http://keyed.mesh:3000", n)` accepts

#### Scenario: Same-origin sibling does not receive the response

- GIVEN two inserted cards, both from `http://keyed.mesh:3000`, and the first sent the pending request
- WHEN the visitor approves
- THEN only the first card's window receives `identity.response`

#### Scenario: Silent sharing after earlier approval is disclosed

- GIVEN `http://keyed.mesh:3000` was approved earlier through `/assert`
- WHEN the card sends `identity.request` with `prompt: none`
- THEN a token is posted with no sheet shown, and the card header in hello.mesh reads "Identity shared with http://keyed.mesh:3000"

#### Scenario: Silent request without approval

- GIVEN an audience that was never approved
- WHEN the card sends `identity.request` with `prompt: none`
- THEN it receives error `interaction_required` and no sheet is shown

#### Scenario: Response never goes to a navigated frame

- GIVEN a card that navigated to `http://other.mesh` after requesting identity
- WHEN the visitor approves
- THEN the pending record is already invalid and nothing is posted
