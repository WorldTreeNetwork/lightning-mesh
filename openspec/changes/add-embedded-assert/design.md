# Design: identity over the mini-app bridge

## Threats and the answer to each

| Threat | Answer |
|---|---|
| App claims to be another audience | Audience is taken from `event.origin`, which the browser sets. The message has no `audience` field. |
| Frame navigates to an attacker origin after the request | The response goes out with `targetOrigin = entryOrigin`, so the browser drops it for any other origin. The receive side also requires `event.origin === entryOrigin`. |
| Some other window posts a request | `event.source` must be this card's `contentWindow`. |
| App draws a fake consent inside its card | Consent that counts is only ever drawn by hello.mesh. The app gets a token only through the host. A fake in-frame "Allow" does nothing. |
| Attacker page frames hello.mesh and clickjacks Allow | `frame-ancestors 'self'` on hello.mesh HTML, and the consent UI refuses to render when `window.top !== window`. |
| Replay of a token | Unchanged from v1: app-minted single-use nonce plus a 300 s expiry. |
| Request spam, or a prompt-storm to fatigue the visitor | At most one open sheet per card. Requests while one is pending are dropped. |
| Self-claimed manifest name ("Bank of Mesh") | The sheet leads with the origin; the manifest name is labelled as what the app calls itself. |
| Signing oracle | The host signs only the fixed v1 payload shape. No app-supplied bytes are signed. |

## Wire

```json
// app → host
{ "mesh": "mini-app/v1", "type": "identity.request", "nonce": "<16–128 hex>", "prompt": "consent" }

// host → app (success)
{ "mesh": "mini-app/v1", "type": "identity.response", "nonce": "<echo>", "token": "<base64url>" }

// host → app (failure)
{ "mesh": "mini-app/v1", "type": "identity.response", "nonce": "<echo>", "error": "access_denied" }
```

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

## Open questions for advise

- Should `frame-ancestors 'self'` also cover the captive-portal page? It is
  opened by the OS sheet, never framed, so the proposal leaves it alone.
- Should `prompt: none` in a card require a visible "signed in to Keyed"
  indicator in hello.mesh chrome? Proposed: yes, as a small badge on the
  card header. The shelf change owns the pixels.
