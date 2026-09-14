# add-embedded-assert

> **ACTIVE BUILD**

Bead `mjolnir-mesh-ncy.4` (epic `mjolnir-mesh-ncy`). Human activated 2026-09-13.
Depends on `add-mini-app-contract` (bridge envelope, insertion rules).
Steer (2026-09-13, recommended default): a `postMessage` transport for the
unchanged `mjolnir-identity-assert` v1 assertion; consent is drawn by the
hello.mesh parent, never inside the frame.

## Why

`/assert` hands a signed, self-verifying identity to another `.mesh` origin
through a full-page redirect. A mini-app inside a hello.mesh card can't use
that: a redirect would navigate the frame, and the sandbox denies top
navigation. Visitors should be able to share their IdentiKey with an
embedded app in one tap, without that app ever seeing the key and without a
new token format for app authors to verify.

## What

- Bridge types `identity.request` (app to host: `nonce`, `prompt`) and
  `identity.response` (host to app: `token` or `error`).
- The **audience is the frame's entry origin as seen in `event.origin`**.
  The app never supplies it, so it can't be spoofed. There is no
  `return_to`.
- Same payload, same domain-separated signature, same 300 s expiry, same
  per-audience approvals store as `/assert`. Apps verify with the existing
  `verifyAssertion`.
- Consent is rendered by hello.mesh outside the iframe. It shows the origin
  as the authority and the manifest name as a secondary, self-claimed label.
- Clickjacking hardening in the other direction: hello.mesh HTML is
  served with `frame-ancestors 'self'`, and the consent UI refuses to render
  when hello.mesh itself is framed.

## Impact

- Capabilities: ADDED to `mesh-mini-apps`. The identity protocol doc gains
  a transport addendum, with Duke's sign-off.
- ADRs: `design.md` in this change (auth transport)

## User journey & surfaces

A visitor who already made an IdentiKey on hello.mesh opens the Keyed card on
the Apps shelf. Keyed's Sign in button asks the host for identity.

1. **Working.** hello.mesh shows a consent sheet over the page, outside the
   card: "`http://keyed.mesh:3000` (calls itself Keyed) wants to know you're
   *Duke* · `a270e81d`". Allow sends the token to the card, and Keyed shows
   the visitor signed in. Next time, `prompt: none` signs in silently.
2. **Empty.** No key yet: the sheet offers "Create your identity first" and
   scrolls to the Identity section. The card gets `interaction_required`
   only if the visitor dismisses it.
3. **Failed.** Deny sends `access_denied`. A malformed nonce sends
   `invalid_request`. A request from the wrong origin, or while a sheet is
   already open for that card, is dropped silently.
4. **Off.** App opened as a link-out tab, not a card: no bridge. The app
   uses the existing `/assert` redirect.

## Out of scope

- Link-out apps: they keep `/assert` unchanged
- Server-side sessions at hello.mesh (none, as today)
- Rung-2/3 custody or hardware keys — `user-identity.md` open items
- Sharing anything beyond pubkey and display name (guild, coordinates) — later
- SDK wrapper `requestIdentity()` — `add-mesh-app-sdk` (`ncy.5`)
