# Cross-Origin Identity Assertion (`mjolnir-identity-assert` v1)

**Status:** Built (frontend) — bead `mjolnir-mesh-1f2`
**Resolves:** open item #3 in [user-identity.md](user-identity.md) §6
**Implementation:** `hello-mesh-web/src/lib/identity/assert.ts` (pure protocol +
reference verifier), `hello-mesh-web/src/routes/assert/+page.svelte` (the
consent/handoff page)

## 1. Problem

A rung-1 browser-held key lives in the IndexedDB of **one origin** (`hello.mesh`).
The same-origin policy makes it invisible to every other origin, so `wiki.mesh`
and `chat.mesh` cannot read it (user-identity.md §4.2). To let a visitor prove
"I am this key + this name" to another `.mesh` service without that service ever
touching the key, `hello.mesh` acts as an OIDC-style identity provider: the
relying party (RP) redirects the browser to `hello.mesh`, the user consents, and
`hello.mesh` signs a **self-certifying assertion** the RP verifies **offline**.

Self-certifying = the signing public key is carried *inside* the signed payload,
so the RP needs no callback to `hello.mesh` and no shared secret. It only needs
the visitor's own claim that this key is theirs — which is exactly what rung-1
identity is.

## 2. Roles

- **Identity provider (IdP):** `hello.mesh` — owns the key, renders consent,
  signs assertions.
- **Relying party (RP):** any other `.mesh` origin that wants to know who the
  visitor is. Mints a nonce, redirects to the IdP, verifies the returned token.

## 3. Wire format

### 3.1 Request (RP → IdP)

The RP redirects the browser to:

```
http://hello.mesh/assert
    ?audience=<RP origin>          # e.g. http://wiki.mesh — a BARE origin, no path
    &return_to=<url>               # where to send the browser back; origin MUST === audience
    &nonce=<hex>                   # 8–64 bytes hex (16–128 hex chars), single-use, RP-minted
    &prompt=consent|none           # optional, default "consent"
```

Validation performed by the IdP (`parseAssertRequest`):

- `audience`, `return_to`, `nonce` are all required.
- `audience` must parse as a URL and equal its own origin (no path/query/fragment).
- `return_to` must be a URL whose **origin === audience**. This is what stops an
  attacker from redirecting the signed token to a host they control.
- `nonce` must match `^[0-9a-fA-F]+$`, even length, 16–128 chars.
- `prompt` must be `consent` or `none` (default `consent`).

Any failure → the IdP shows an on-page "invalid request" error. It does **not**
redirect, because on a validation failure `return_to`/`audience` are precisely
the untrusted inputs that failed.

### 3.2 Payload (what gets signed)

A JSON object with **exact key order** (both signer and verifier build it via
`buildAssertionPayload`, so the serialized bytes agree):

```json
{
  "v": 1,
  "pubkey": "<64-hex ed25519 public key>",
  "display_name": "<string>",
  "audience": "<RP origin>",
  "nonce": "<hex, echoed from request>",
  "issued_at": <unix secs>,
  "expires_at": <issued_at + 300>
}
```

Assertions live for **300 seconds** (`ASSERT_TTL_SECS`). This is a login handoff,
not a session — the RP mints its own session after verifying.

### 3.3 Signature

```
sig = ed25519(secretKey, utf8("mjolnir-identity-assert:v1\n" + payloadJson))
```

The domain-separation prefix (`ASSERT_DOMAIN`) mirrors `name_claim_signing_message`
in `crates/mjolnir-hello/src/routes.rs` — prefix, newline, then the exact payload
bytes. Domain separation guarantees a signature minted for one protocol (e.g. a
name claim) can never be replayed as an assertion and vice-versa.

`sig` is the 128-hex encoding of the 64-byte Ed25519 signature.

### 3.4 Token & return (IdP → RP)

The token is `base64url(JSON.stringify({payload: payloadJson, sig: <128-hex>}))`.
Carrying the exact `payload` bytes (not a re-serialization) means the RP verifies
over precisely what was signed, sidestepping any JSON canonicalization concern.

On **approve**, the IdP redirects to:

```
<return_to>#mjolnir_assertion=<base64url token>
```

On **deny / silent-failure**, the IdP redirects to:

```
<return_to>#mjolnir_assertion_error=<code>
```

where `<code>` ∈ `access_denied` | `interaction_required` | `invalid_request`.

**The token is carried in the URL FRAGMENT, never the query.** Fragments are not
transmitted to any server, so the signed assertion never lands in a node's HTTP
logs or the RP's server-side request. The RP reads it from `location.hash` in
client JS.

## 4. Consent & silent re-auth

- Consent is remembered per audience in `localStorage` under
  `hello-mesh-assert-approvals` — a JSON object `{ <audience>: <approved_at unix> }`.
- `prompt=consent` (default): the IdP **always** shows the consent screen (display
  name + first 8 hex of the pubkey + which audience is asking). Approve signs and
  redirects; Deny redirects with `access_denied`.
- `prompt=none`: no UI. If the visitor already has a key **and** has previously
  approved this audience, the IdP signs and redirects immediately. Otherwise it
  redirects back with `interaction_required` (the RP then re-requests with
  `prompt=consent`). This is the standard OIDC silent-refresh dance.
- No key yet + `prompt=consent`: the IdP shows a "create your identity first"
  panel linking to the front desk (`/`); the visitor makes a key and returns.

## 5. RP-side verification (offline)

Reference implementation: `verifyAssertion` in `assert.ts`, exported so RP authors
can copy it. Steps:

1. Read `location.hash`; pull `mjolnir_assertion` (or handle `mjolnir_assertion_error`).
2. `base64url`-decode → `JSON.parse` → `{ payload, sig }`; `JSON.parse(payload)`.
3. Verify `payload.v === 1` and `payload.pubkey` is 64-hex.
4. Verify `sig` over `utf8("mjolnir-identity-assert:v1\n" + payload)` using
   **`payload.pubkey`** (self-certifying — the key is inside the token).
5. Check `payload.audience === <your own origin>`.
6. Check `payload.nonce === <the nonce you minted>` and that you have not already
   consumed it (single-use — this is your replay defense).
7. Check `now < payload.expires_at`.

All checks pass → trust `payload.pubkey` as the visitor's identity and
`payload.display_name` as their name; mint your own session.

### 5.1 RP verifier example (`@noble/ed25519`)

```ts
import * as ed from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha2.js';

// Insecure-context (plain HTTP) safety — noble must not touch crypto.subtle.
ed.hashes.sha512 = sha512;

const DOMAIN = 'mjolnir-identity-assert:v1';

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  return out;
}
function b64urlToBytes(s: string): Uint8Array {
  const pad = s.length % 4 ? '='.repeat(4 - (s.length % 4)) : '';
  const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

export function verify(encoded: string, myOrigin: string, expectedNonce: string) {
  const { payload, sig } = JSON.parse(new TextDecoder().decode(b64urlToBytes(encoded)));
  const claim = JSON.parse(payload);
  const msg = new TextEncoder().encode(`${DOMAIN}\n${payload}`);
  if (!ed.verify(hexToBytes(sig), msg, hexToBytes(claim.pubkey))) throw new Error('bad sig');
  if (claim.audience !== myOrigin) throw new Error('audience mismatch');
  if (claim.nonce !== expectedNonce) throw new Error('nonce mismatch'); // + mark consumed
  if (Math.floor(Date.now() / 1000) >= claim.expires_at) throw new Error('expired');
  return { pubkey: claim.pubkey, displayName: claim.display_name };
}
```

## 6. Edge cases & lifecycle

- **Name changes.** The assertion carries `display_name` as a convenience snapshot;
  the *authoritative* name lives in the directory CRDT (`0yb`), keyed by pubkey. An
  RP that wants the current name looks the pubkey up in the directory rather than
  trusting a stale assertion. The pubkey is the stable identifier.
- **Key rotation = new identity.** Rung-1 has no rotation-in-place: a new key is a
  new identity (new directory row). An RP keyed on pubkey simply sees a different
  user; re-linking accounts across a key change is an RP/directory concern, out of
  scope here.
- **Replay.** Defended by the single-use nonce (RP-enforced) plus the 300 s expiry.
  The fragment-only return keeps the token off the wire in the first place.
- **Redirect hardening.** `return_to`'s origin must equal `audience`, so a signed
  assertion for `wiki.mesh` can only be delivered to a `wiki.mesh` URL.

## 7. Non-goals

- **No signing oracle.** `hello.mesh` signs only this fixed assertion shape after
  explicit (or previously-granted) consent — it will not sign arbitrary RP-supplied
  bytes.
- **No key export to the RP.** The RP receives a public key and a signature, never
  the secret key or recovery phrase. Soft custody (user-identity.md §3) is unchanged.
- **No server-side sessions at the IdP.** `hello.mesh` holds no per-RP session and
  no token log; the whole exchange is a stateless redirect. The RP owns session
  lifetime after verification (open item #4).
- **Not a higher trust rung.** This is rung-1 soft custody: the serving node could
  extract the key. The assertion honestly conveys that tier; custodial attestation
  (rung 3) is a separate protocol (open item #2).
