# Tasks

- [ ] Advise review of this change and `design.md` (reader Fable 5.1); accept before code
- [ ] `hello-mesh-web/src/lib/miniapp/identity-bridge.ts`: pure `decideIdentityRequest(msg, entryOrigin, hasKey, approvals, pending)` returning `prompt`, `sign`, `error`, or `drop`
- [ ] Reuse `buildAssertionPayload` / `signAssertion` / `encodeAssertionToken` from `lib/identity/assert.ts`; no second payload builder
- [ ] Share the approvals store (`hello-mesh-assert-approvals`) with `/assert` behind one accessor
- [ ] `identity-bridge.spec.ts`: one test per scenario in `specs/mesh-mini-apps/spec.md`, plus a round trip through `verifyAssertion`
- [ ] Consent sheet component, rendered in the hello.mesh document with the origin as the primary label
- [ ] Frame refusal: consent UI and `/assert` do not render approve controls when `window.top !== window`
- [ ] `crates/mjolnir-hello`: `Content-Security-Policy: frame-ancestors 'self'` on HTML responses (not on `/api/*` or the captive-portal probe payloads), with a routes test
- [ ] Propose the transport addendum for `docs/network-coordination/identity-assertion.md` and get Duke's sign-off before editing it
- [ ] `bun run check`, `bun run test`, `CARGO_TARGET_DIR=/tmp/lm-target cargo test -p mjolnir-hello` green
