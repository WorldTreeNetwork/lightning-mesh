# Tasks

- [ ] Advise review of this change and `design.md` (reader sol-arch-review, other family); accept before code
- [ ] `hello-mesh-web/src/lib/miniapp/identity-bridge.ts`: pure `decideIdentityRequest(msg, entryOrigin, hasKey, approvals, pending)` returning `prompt`, `sign`, `error`, or `drop`
- [ ] Reuse `buildAssertionPayload` / `signAssertion` / `encodeAssertionToken` from `lib/identity/assert.ts`; no second payload builder
- [ ] Share the approvals store (`hello-mesh-assert-approvals`) with `/assert` behind one accessor
- [ ] `identity-bridge.spec.ts`: one test per scenario in `specs/mesh-mini-apps/spec.md`, plus a round trip through `verifyAssertion`
- [ ] Consent sheet component, rendered in the hello.mesh document with the origin as the primary label
- [ ] Frame refusal, fail-closed ordering: when `window.top !== window` (any parent, including a hello.mesh origin), the bridge and `/assert` exit before identity load, approval reads, listener registration or signing. Tests cover foreign framing and same-origin self-framing for the bridge and a previously approved `/assert?prompt=none`.
- [ ] Port-bound delivery: `identity.request` must transfer exactly one `MessagePort`; token and errors go only on it, and no token is ever sent through `window.postMessage`. Test a replacement same-origin document that is listening before the parent's `load` and gets no token.
- [ ] Pending lifecycle: 60 s host-owned expiry and per-card cooldown (30 s doubling to 10 min, reset by visitor approval or re-open). Test that B gets consent after A expires and that A can't re-acquire the slot in a deny loop.
- [ ] `crates/mjolnir-hello`: `Content-Security-Policy: frame-ancestors 'self'` on HTML responses (not on `/api/*` or the captive-portal probe payloads), with a routes test
- [ ] Propose the transport addendum for `docs/network-coordination/identity-assertion.md` and get Duke's sign-off before editing it
- [ ] `bun run check`, `bun run test`, `CARGO_TARGET_DIR=/tmp/lm-target cargo test -p mjolnir-hello` green

## Owed from advise (sol-arch-review, 2026-09-13)

- [ ] Amend the contract and tests so framed hello.mesh fails closed before bridge registration, identity/approval reads, or signing; cover previously approved `prompt:none` in both the bridge and `/assert`
- [ ] Bind each pending request and response to the exact requesting card/window, invalidate it on iframe load/removal/replacement, and test a same-origin sibling plus navigate-away-and-back before approval
- [ ] Define globally safe consent concurrency (one global pending request or an explicit safe queue), bind Allow to the displayed source/origin/nonce tuple, and test simultaneous requests from different cards
- [ ] Make the shelf-owned trusted-chrome “Identity shared with <origin>” indicator a shipping dependency for successful embedded `prompt:none`; do not claim app-session success without app confirmation
- [ ] Reconcile CSP scope as SPA/static hello.mesh application HTML only, explicitly excluding captive `PORTAL_HTML` and probe success bodies, with presence/absence and captive round-trip regression tests

## Owed from advise round 2 (sol-arch-review, 2026-09-13)

- [ ] Replace load-event-only request invalidation with a document-bound response mechanism (or equivalent pre-delivery proof) that cannot survive iframe navigation; test a replacement same-origin document that can receive messages before the parent `load` handler and prove it gets no token
- [ ] Bound the one-global-pending lifecycle with host-owned expiry/cancellation and re-prompt control; test that one hostile card cannot indefinitely block or denial-loop another card's consent request
- [ ] Consolidate the frame-refusal task around fail-closed ordering, and add same-origin hello.mesh self-frame tests for both the bridge and previously approved `/assert?prompt=none`

## Owed from advise round 3 (sol-arch-review, 2026-09-13)

- [ ] Replace the unenforceable “only the requesting document holds/receives through the port” claim with the precise capability boundary: the host delivers once only to the port designated on the authenticated card request; a replacement document does not inherit it, while deliberate requester delegation cannot be detected or prevented
- [ ] Make the response port strictly one-way: all app-to-host identity input is accepted only on the original source-and-origin-checked window event; never interpret incoming port messages; send at most one terminal response and close. Test forged port follow-ups cannot alter audience, nonce, pending ownership or consent and cannot cause a second response
- [ ] Pin lifecycle event ordering with an absolute host deadline checked on every request/decision (expiry wins at or after the deadline; a pre-deadline Allow atomically claims the record), and define cooldown re-open reset as a direct trusted shelf gesture only. Test a delayed expiry timer versus Allow/B requests and prove iframe navigation, bridge traffic and app-driven teardown/recreation cannot reset cooldown
- [ ] Add a MODIFIED `Bridge envelope` delta on this change (do not edit `add-mini-app-contract`): extend v1 with window-carried `identity.request` and port-carried `identity.response`; retain exact source/origin plus serialization/16-KiB checks for the request; retain exact `targetOrigin` for window sends; specify that the one-shot response port is the sole identity-response exception and identity tokens/errors are never window messages
