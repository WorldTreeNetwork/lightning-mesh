## ADDED Requirements

### Requirement: Identity request over the bridge

hello.mesh SHALL accept an `identity.request` bridge message only from a
registered inserted card, meaning `event.source` is that card's iframe window
and `event.origin` is its entry origin. A `null` origin is never accepted.
The message SHALL carry `nonce` (even-length hex, 16–128 chars) and `prompt`
(`consent` or `none`, default `consent`). It SHALL also transfer exactly
one `MessagePort` created by the requesting document. The assertion audience
SHALL be the card's entry origin, and the message SHALL NOT be able to
supply a different audience. A request without exactly one port SHALL be
dropped without a response. A malformed request that carries a port SHALL
be answered on that port with error `invalid_request`, echoing the nonce
only when it is well formed.

hello.mesh SHALL hold at most one pending identity request across all cards.
The pending record SHALL capture the card, its entry origin, the nonce and
the transferred port. It SHALL end, with the sheet closed, no signing, and
the port closed, when any of these happens:
- the visitor approves, denies or dismisses the sheet
- 60 seconds pass without a decision (host-owned expiry)
- the iframe fires `load` again, is removed, or is replaced

While a request is pending, further requests from the same card SHALL be
dropped without a response. Requests from any other card SHALL be answered
at once on their own port with error `interaction_required`.

A card whose request ends without approval (deny, dismiss or expiry) SHALL
enter a cooldown. During it, that card's requests are answered at once with
`interaction_required` and no sheet opens. The cooldown SHALL be 30 seconds
and SHALL double on each consecutive unapproved ending, up to 10 minutes. It
SHALL reset when the visitor approves a request from that card, or when the
visitor re-opens the card from the shelf.

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

#### Scenario: Duplicate request from the same card

- GIVEN a pending request from a card
- WHEN that card sends another `identity.request`
- THEN no second sheet opens and no response is sent for the second request

#### Scenario: Two cards request at once

- GIVEN cards A and B are both inserted and A's request is pending
- WHEN B sends `identity.request`
- THEN B receives error `interaction_required` on its port, and the open sheet still names A's origin and is bound to A's nonce

#### Scenario: Hostile card cannot hold the slot

- GIVEN card A opened a request and the visitor walks away
- WHEN 60 seconds pass and card B then sends `identity.request`
- THEN A's sheet has closed, A's port received `interaction_required`, and B's sheet opens

#### Scenario: Denial loop is throttled

- GIVEN the visitor denied card A's request
- WHEN A sends `identity.request` again 5 seconds later
- THEN no sheet opens and A receives `interaction_required`, and after a second unapproved ending A's cooldown is 60 seconds

#### Scenario: Navigate away and back invalidates the request

- GIVEN a pending request from a card at `http://keyed.mesh:3000`
- WHEN the iframe navigates to `http://other.mesh` and then back to `http://keyed.mesh:3000` before the visitor approves
- THEN the sheet closes, nothing is signed, and no token is sent

### Requirement: Consent rendered by the host

For `prompt: consent`, hello.mesh SHALL render consent in its own document,
outside the iframe. The consent SHALL show the entry origin as the primary
label, the manifest name only as a label for what the app calls itself, the
visitor's display name, and the first 8 hex chars of the pubkey. The Allow
action SHALL sign only for the pending record the sheet displays (card,
origin, nonce, port), and only if that record is still pending when Allow
runs. When the visitor has no key, consent SHALL offer to create one first.

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

On approve, hello.mesh SHALL send `identity.response`, carrying the echoed
nonce and a token built exactly as `mjolnir-identity-assert` v1 (same
payload key order, domain prefix, 300-second expiry, base64url encoding),
**only on the port transferred with the pending request**, and then close
that port. It SHALL NOT send an identity token through `window.postMessage`
to any window. On deny or dismiss it SHALL send error `access_denied` on
the port.

For `prompt: none` it SHALL sign without a sheet only when a key exists, the
audience is already in the approvals store shared with `/assert`, and the
card is not in cooldown. It SHALL otherwise answer `interaction_required` on
the port. Every successful `prompt: none` issuance SHALL show "Identity
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
- THEN only the first card's port receives `identity.response`

#### Scenario: Replacement document gets no token before load fires

- GIVEN a pending request whose port belongs to the card's current document
- WHEN the iframe navigates to a new `http://keyed.mesh:3000` document that is already listening for messages, and the visitor clicks Allow before the parent sees `load`
- THEN the new document receives no token, because the token goes only to the original document's port

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
