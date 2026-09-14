## ADDED Requirements

### Requirement: Identity request over the bridge

hello.mesh SHALL accept an `identity.request` bridge message from an
inserted mini-app carrying `nonce` (even-length hex, 16–128 chars) and
`prompt` (`consent` or `none`, default `consent`). The assertion audience
SHALL be the card's entry origin as reported by `event.origin`, and the
message SHALL NOT be able to supply a different audience. A malformed request
SHALL be answered with `identity.response` error `invalid_request`, echoing
the nonce only when it is well formed. While a consent sheet is open for a
card, further requests from that card SHALL be dropped without a response.

#### Scenario: Audience comes from the origin

- GIVEN a card inserted from `http://keyed.mesh:3000`
- WHEN it sends `identity.request` with an extra field `audience: "http://bank.mesh"`
- THEN any assertion issued has audience `http://keyed.mesh:3000`

#### Scenario: Malformed nonce

- GIVEN an inserted card
- WHEN it sends `identity.request` with nonce `xyz`
- THEN it receives `identity.response` with error `invalid_request` and no nonce

#### Scenario: Request while a sheet is open

- GIVEN a consent sheet already open for a card
- WHEN that card sends another `identity.request`
- THEN no second sheet opens and no response is sent for the second request

### Requirement: Consent rendered by the host

For `prompt: consent`, hello.mesh SHALL render consent in its own document,
outside the iframe. The consent SHALL show the entry origin as the primary
label, the manifest name only as a label for what the app calls itself, the
visitor's display name, and the first 8 hex chars of the pubkey. When the
visitor has no key, consent SHALL offer to create one first. hello.mesh
SHALL NOT render approve controls when its own window is framed
(`window.top !== window`), and its HTML responses SHALL carry
`Content-Security-Policy: frame-ancestors 'self'`.

#### Scenario: Consent names the origin first

- GIVEN a card from `http://keyed.mesh:3000` whose manifest name is "Bank of Mesh"
- WHEN it requests identity with `prompt: consent`
- THEN the sheet leads with `http://keyed.mesh:3000` and shows "Bank of Mesh" as what the app calls itself

#### Scenario: hello.mesh framed by another page

- GIVEN an attacker page that iframes `http://hello.mesh/`
- WHEN the browser loads it
- THEN the frame is blocked by `frame-ancestors 'self'`, and if rendered anyway no approve control appears

#### Scenario: No key yet

- GIVEN a visitor with no IdentiKey
- WHEN a card requests identity with `prompt: consent`
- THEN the sheet offers to create an identity, and dismissing it sends error `interaction_required`

### Requirement: Identity response over the bridge

On approve, hello.mesh SHALL post `identity.response` carrying the echoed
nonce and a token built exactly as `mjolnir-identity-assert` v1 (same
payload key order, domain prefix, 300-second expiry, base64url encoding),
with `targetOrigin` equal to the entry origin. On deny it SHALL post error
`access_denied`. For `prompt: none` it SHALL sign without UI only when a key
exists and the audience is already in the approvals store shared with
`/assert`, and SHALL otherwise post error `interaction_required`. Approving
in either transport SHALL record the audience in that shared store.

#### Scenario: Approve yields a verifiable token

- GIVEN a visitor with a key and a card from `http://keyed.mesh:3000` that sent nonce `n`
- WHEN the visitor approves
- THEN the card receives a token that `verifyAssertion(token, "http://keyed.mesh:3000", n)` accepts

#### Scenario: Silent sign-in after earlier approval

- GIVEN `http://keyed.mesh:3000` was approved earlier through `/assert`
- WHEN the card sends `identity.request` with `prompt: none`
- THEN a token is posted with no sheet shown

#### Scenario: Silent request without approval

- GIVEN an audience that was never approved
- WHEN the card sends `identity.request` with `prompt: none`
- THEN it receives error `interaction_required` and no sheet is shown

#### Scenario: Response never goes to a navigated frame

- GIVEN a card that navigated to `http://other.mesh` after requesting identity
- WHEN the visitor approves
- THEN the response is posted with targetOrigin `http://keyed.mesh:3000` and is not delivered
