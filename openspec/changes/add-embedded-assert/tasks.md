# Tasks

- [ ] Advise review of this change and `design.md` (reader sol-arch-review, other family); accept before code
- [ ] `hello-mesh-web/src/lib/miniapp/identity-bridge.ts`: pure `decideIdentityRequest(msg, entryOrigin, hasKey, approvals, pending)` returning `prompt`, `sign`, `error`, or `drop`
- [ ] Reuse `buildAssertionPayload` / `signAssertion` / `encodeAssertionToken` from `lib/identity/assert.ts`; no second payload builder
- [ ] Share the approvals store (`hello-mesh-assert-approvals`) with `/assert` behind one accessor
- [ ] `identity-bridge.spec.ts`: one test per scenario in `specs/mesh-mini-apps/spec.md`, plus a round trip through `verifyAssertion`
- [ ] Consent sheet component, rendered in the hello.mesh document with the origin as the primary label
- [ ] Frame refusal: consent UI and `/assert` do not render approve controls when `window.top !== window`
- [ ] `crates/mjolnir-hello`: `Content-Security-Policy: frame-ancestors 'self'` on HTML responses (not on `/api/*` or the captive-portal probe payloads), with a routes test
- [ ] Propose the transport addendum for `docs/network-coordination/identity-assertion.md` and get Duke's sign-off before editing it
- [ ] `bun run check`, `bun run test`, `CARGO_TARGET_DIR=/tmp/lm-target cargo test -p mjolnir-hello` green

## Owed from advise (sol-arch-review, 2026-09-13)

- [ ] Amend the contract and tests so framed hello.mesh fails closed before bridge registration, identity/approval reads, or signing; cover previously approved `prompt:none` in both the bridge and `/assert`
- [ ] Bind each pending request and response to the exact requesting card/window, invalidate it on iframe load/removal/replacement, and test a same-origin sibling plus navigate-away-and-back before approval
- [ ] Define globally safe consent concurrency (one global pending request or an explicit safe queue), bind Allow to the displayed source/origin/nonce tuple, and test simultaneous requests from different cards
- [ ] Make the shelf-owned trusted-chrome “Identity shared with <origin>” indicator a shipping dependency for successful embedded `prompt:none`; do not claim app-session success without app confirmation
- [ ] Reconcile CSP scope as SPA/static hello.mesh application HTML only, explicitly excluding captive `PORTAL_HTML` and probe success bodies, with presence/absence and captive round-trip regression tests
