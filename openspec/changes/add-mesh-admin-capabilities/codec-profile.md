# Local mesh administrative profile v1 — implementation pin

Status: proposed local pin under this ACTIVE change, not upstream conformance.
Independent review of this pin is required before the codec slice acts.

## Sources and explicit compatibility boundary

IdentiKey protocol revision `093ceb13148d5fb5267ad309ecdfea15f2637d9e`:
`identikey-auth-challenge-v1.md` sections 3–5 provides self-describing public keys,
algorithm-committing fingerprints and deterministic CBOR conventions. Reuse those
primitives, not the auth-challenge signing domain. The separate recrypt identity
envelope fingerprint is a different namespace; do not substitute it. The upstream
capability tuple/vectors are still owed to `ikp-6yz.2`. This document is the local
mesh normative tuple until explicitly versioned reconciliation, tracked on ai0.1.

CBOR implementation pin: `minicbor = "=0.26.5"`, feature `std`, verified through
the downloaded crates.io source (Encoder/Decoder typed array, bytes, str, u64,
position APIs). It supplies CBOR primitives; strictness is enforced by our schema
and exact decode/re-encode comparison, not a claim that minicbor enforces dCBOR.
Only unsigned integers, fixed ASCII strings, definite arrays and byte strings
occur in requests. No floats, tags, maps, null, optional fields or user text.

Holder crypto pin: `ed25519-dalek = "=2.2.0"`; use
[`VerifyingKey::from_bytes` and `verify_strict`](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/struct.VerifyingKey.html),
reject weak keys, never permissive verification or caller-supplied proof booleans.
V1 accepts only `ed25519`, 32-byte public key and 64-byte signature. P-256 and PQ
are unsupported in this local v1 profile, not silently downgraded. This does not
claim enclave/passkey integration; extending algorithms requires a reviewed pin.

## Canonical request and proof

The signed payload is the following definite 10-element CBOR array, with integers
in their smallest representation and every byte string definite-length:

```
["lightning-admin/request", 1,
 house_id: bstr32, node_id: bstr32, authority_epoch: uint,
 operation: uint, arguments: array,
 token_identity: bstr32, target_challenge: bstr32, request_id: bstr32]
```

`house_id` and `node_id` are opaque identifiers supplied by verified authority
state; the codec does not derive or authenticate them. Epoch is 0..i64::MAX for
Datalog compatibility. Challenges and request IDs must be generated with a CSPRNG
by the destination/control layer; parsing 32 bytes alone does not prove freshness.

Operations are a closed enum:
- 0 = diagnostics status, arguments exactly `[]`.
- 1 = apply a pre-staged network plan, arguments exactly
  `[plan_id: bstr32, expected_revision: bstr32, plan_digest: bstr32]`.

Operation 1 describes a future adapter, not permission or a built operation. Its
consumer must validate the complete stored plan against the signed ID/revision/
digest immediately before execution. The plan digest's canonical preimage belongs
to durable apply and must exclude plaintext secrets exposed to clients (use opaque
secret references). No arbitrary shell commands or unsigned extra arguments.

Reject unknown version/domain/operation, wrong arity/length/type, trailing bytes,
indefinite/nonminimal encodings, and payloads exceeding 1024 bytes before allocation
or parsing. Decode into typed fields, encode with the one encoder, and compare
exactly to the input. No normalization of hostile bytes followed by verification.

Token identity resolves review B1:
`BLAKE3(ASCII("lightning-admin/token/v1") || 0x00 || raw_biscuit_bytes)`.
Raw token size is 1..65536 bytes. It covers the exact complete received binary
token, including attenuations; decode any transport base64 first, but do not
parse/re-serialize the token before hashing. Attenuation or sealing changes its
identity and requires a fresh holder signature. This is NOT a revocation-lineage
identifier; the authorization engine separately checks revoked ancestor IDs.

Fingerprint resolves B4 using auth-challenge v1 section 5:
`BLAKE3(CBOR({"alg": "ed25519", "key": bstr32}))`, with map keys `alg`, `key`
in that encoded order. The holder signature covers the exact canonical request
bytes, without another prehash/signing domain. The versioned array is its domain.
Proof verification checks the signature and recomputed token identity. Its result
is explicitly `VerifiedHolderProof`, not `AuthorizedRequest`: it establishes only
possession of a key for those bytes. Issuer/rights/holder grant binding, trusted
time, revocation, target-challenge matching and durable one-use execution are
separate mandatory checks; the codec must not expose an execute/authorize API.

## Owner-signed records (`add-control-record-integrity`)

Records reuse this profile and the same gate (size ceiling, closed-schema
decode, no trailing bytes, exact re-encode). There is no second canonicalizer.
Each record is a definite CBOR array opened by its own domain string, version 1,
and a shared header:

```
[domain, 1, house_id: bstr32, authority_epoch: uint, signer: bstr32,
 sequence: uint, issued_at: uint, subject: bstr32, ...body]
```

| Domain | Arity | `subject` | Body |
|---|---|---|---|
| `lightning-admin/record/head` | 12 | previous head digest (zero for genesis) | `owner: bstr32, profile: uint, issuers: [bstr32], revoked_lineage: [bstr]` |
| `lightning-admin/record/snapshot` | 10 | head digest it chains to | `issuers: [bstr32], revoked_lineage: [bstr]` |
| `lightning-admin/record/withdrawal` | 8 | withdrawn issuer key | none |
| `lightning-admin/record/control` | 11 | node id | `grant_class: uint, token_root: bstr32, expires_at: uint` |

Profile codes are 0 strict, 1 standard and 2 relaxed. Grant-class codes are
0 diagnostics-read, 1 network-radio, 2 firmware, 3 ownership and 4 guest-wifi.
Issuers and revoked lineage are strictly ascending. There are at most 64
issuers and 32 lineage ids of 1–128 bytes, and a record is at most 8192 bytes.
`issued_at` and `expires_at` are at most `i64::MAX`, and `expires_at` must be
after `issued_at`. The signature is Ed25519 `verify_strict` by `signer` over
exactly these bytes, with weak keys rejected. A head names its predecessor by
`BLAKE3(ASCII("lightning-admin/record-digest/v1") || 0x00 || canonical_bytes)`.

The persisted consumer freshness state (`lightning-admin/state/record-book`,
version 1) is local and never gossiped. It is decoded through the same gate,
and every entry is re-verified on restore.

## Authorization-engine pins (not implemented by the codec slice)

Use `biscuit-auth = "=6.0.0"`. Documented
[`Biscuit::from`, `external_public_keys`, `block_count`, `to_vec`](https://docs.rs/biscuit-auth/6.0.0/biscuit_auth/struct.Biscuit.html)
provide verified parsing and block inspection. Never use deprecated unsafe parsing.
Reject all third-party blocks in v1 (any external key). Maximum 16 blocks including
authority, 64 KiB serialized token. Reject reserved verifier-fact assertions in
any block, including rule heads that could derive them; inspect parsed structures,
not substring matching. Root holder check is mandatory. Facts cannot manufacture
current authority, holder, time, request or operation context.

Use explicit authority-only rights policy plus trusted authorizer context;
appended blocks may restrict through checks, never contribute rights. No external
functions. Set [authorizer limits](https://docs.rs/biscuit-auth/6.0.0/biscuit_auth/type.AuthorizerLimits.html)
to 1024 facts, 64 iterations and 20ms evaluation; limit errors deny. These are initial
resource ceilings, not measured performance guarantees. Runtime registration and
parsed-block inspection APIs must be pinned in the engine packet before coding.
Do not use `AuthorizerBuilder::time()` as trusted time evidence: it reads the clock.

Trusted authority/time inputs still require implementation under ai0.1/ai0.9; this
codec slice must not fake them to claim the full grant verifier exists. Absolute
validity stays fail-closed until authenticated time evidence is available.

## First implementation slice

`mjolnir-mesh-ai0.1.1`: new workspace library `mjolnir-admin-capabilities`, canonical
request encoder/strict decoder, token digest, holder fingerprint/proof only.
No public authorization method, token minting or downstream imports yet. Golden
hex fixtures include both operations and integer boundaries; negative fixtures
cover all malformed forms above and every signed-field mutation. Deterministic
test keys are test-only. Tests must distinguish altered-byte rejection from actual
authorization/replay resistance, neither of which this slice implements.

