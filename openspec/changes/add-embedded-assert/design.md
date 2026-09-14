# Design: identity over the mini-app bridge

## Threats and the answer to each

| Threat | Answer |
|---|---|
| App claims to be another audience | Audience is taken from `event.origin`, which the browser sets. The message has no `audience` field. |
| Frame navigates to an attacker origin after the request | The token goes only over the `MessagePort` the **requesting document** transferred, and a port doesn't survive that document. The iframe's `load` also ends the pending record as cleanup. |
| Frame navigates away **and back** to the same origin before approval | A `WindowProxy` survives navigation, and a new same-origin document can run script and listen **before** the parent sees `load`. So neither origin checks nor `load` invalidation is proof. The document-owned port is. (Round-2 send-back.) |
| Same-origin sibling card | `targetOrigin` can't tell two `keyed.mesh` cards apart. The response goes to the pending record's port, never looked up by origin or window. |
| Some other window posts a request | `event.source` must be a registered card's `contentWindow`, and `null` origins are refused. |
| App draws a fake consent inside its card | Consent that counts is only ever drawn by hello.mesh. The app gets a token only through the host. A fake in-frame "Allow" does nothing. |
| Attacker page frames hello.mesh and clickjacks Allow | `frame-ancestors 'self'` on SPA/static HTML, plus a fail-closed `window.top !== window` check that runs **before** identity load, approval lookup, bridge registration or signing. That order matters: today's `/assert` signs `prompt=none` before rendering anything, so hiding controls alone would not stop a framed silent sign. |
| A hello.mesh page frames hello.mesh (same origin) | CSP `'self'` deliberately allows this, so the runtime check is the only defence. It refuses **any** framing, and it's tested for both the bridge and a previously approved `/assert?prompt=none`. |
| Replay of a token | Unchanged from v1: app-minted single-use nonce plus a 300 s expiry. |
| Two cards race one consent sheet (origin confusion at the click) | One global pending request. Other cards get `interaction_required` at once, and Allow signs only the displayed (card, origin, nonce, port) and only while it's still pending. |
| One hostile card holds the global slot forever | Host-owned 60 s expiry ends any pending request. (Round-2 send-back.) |
| Deny, re-request, deny loop to fatigue the visitor or starve other cards | After each unapproved ending the card enters a cooldown with no sheet: 30 s, doubling to 10 min, reset only by the visitor's approval or re-opening the card from the shelf. The app can't reset it. Same-card duplicates while pending are dropped. |
| Silent `prompt:none` becomes invisible ambient sharing | Every silent issuance shows "Identity shared with `<origin>`" in hello.mesh chrome on that card. It doesn't say "signed in", because the host can't know the app made a session. |

(Rows for navigate-back, sibling, race and silent disclosure, plus the
fail-closed ordering, come from the advise send-back of 2026-09-13,
sol-arch-review.)
| Self-claimed manifest name ("Bank of Mesh") | The sheet leads with the origin; the manifest name is labelled as what the app calls itself. |
| Signing oracle | The host signs only the fixed v1 payload shape. No app-supplied bytes are signed. |

## Wire

```js
// app → host: window.parent.postMessage(msg, hostOrigin, [channel.port2])
{ "mesh": "mini-app/v1", "type": "identity.request", "nonce": "<16–128 hex>", "prompt": "consent" }

// host → app, success: sent ONLY on the transferred port, then the port is closed
{ "mesh": "mini-app/v1", "type": "identity.response", "nonce": "<echo>", "token": "<base64url>" }

// host → app, failure: same port
{ "mesh": "mini-app/v1", "type": "identity.response", "nonce": "<echo>", "error": "access_denied" }
```

Why a port: the requesting document creates the `MessageChannel`, so only
that document holds `port1`. If the frame navigates, even back to the same
origin and even before the parent sees `load`, the new document has no
reference to it. Host code never sends identity through
`window.postMessage`, so `targetOrigin` is no longer the delivery guard for
tokens, only for the non-identity v1 types.

The token is byte-identical in shape to the `/assert` fragment token: the
same `buildAssertionPayload` with `audience = entryOrigin`, the same
`mjolnir-identity-assert:v1` domain. An app verifies it with
`verifyAssertion(token, location.origin, nonce)`. Because the audience is
the origin the frame actually runs on, an honest app's own-origin check
matches.

Error codes are reused from v1: `access_denied`, `interaction_required`,
`invalid_request`. The `nonce` is echoed on errors when it was well formed,
so apps can correlate concurrent attempts.

## Why not reuse the redirect inside the frame

A redirect would navigate the frame to `hello.mesh/assert`. That makes the
frame same-origin with hello.mesh, and with `allow-scripts allow-same-origin`
it could reach the parent. It is exactly the configuration
`add-mini-app-contract` forbids. It would also put the key-holding page
inside a frame, which the clickjacking rule refuses. The bridge avoids both.

## Approvals

One store, one accessor, shared with `/assert`. Approving `keyed.mesh:3000`
in a card also lets a link-out `/assert` from that origin go silent, and the
reverse. The audience string is identical in both cases.

## Resolved at advise (2026-09-13)

- **Captive portal and CSP.** Excluded explicitly. `PORTAL_HTML` holds no
  key and no approval control, and it's served at intercepted OS probe
  origins. The header goes on SPA/static application HTML only, with
  regression tests for both presence and absence.
- **Silent indicator.** Required, and it reads "Identity shared with
  `<origin>`", not "signed in". The shelf change (`ncy.3`) owns the pixels,
  so `ncy.4` can't ship before that indicator exists (bead edge added).
- **Revoke UI.** Not a blocker here. Approval revocation is an existing
  `/assert` lifecycle concern.
