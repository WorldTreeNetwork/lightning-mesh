# ai0.1.1 codec slice — red-first evidence

Node `mjolnir-mesh-ai0.1.1`. Tests were written against a **behavioural stub**
(full public API present, every operation returning a typed error) and run to
observe failure *before* any implementation existed. This is behavioural red,
not compile-only failure: the test binary built and ran all 28 tests.

## Command

```
CARGO_TARGET_DIR=/tmp/lm-target cargo test -p mjolnir-admin-capabilities
```

Run from `/home/dorje/work/WorldTree/lightning-mesh` on 2026-09-15.
`--locked` is deliberately omitted here because this run is what first resolves
the new `minicbor =0.26.5` / `ed25519-dalek =2.2.0` pins into `Cargo.lock`; the
acceptance command uses `--locked`.

## Observed output (verbatim, abridged only where marked)

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src/lib.rs (/tmp/lm-target/debug/deps/mjolnir_admin_capabilities-42c97bd1e5e42f01)
     Running tests/codec.rs (/tmp/lm-target/debug/deps/codec-dbd824b9f05148b0)
error: test failed, to rerun pass `-p mjolnir-admin-capabilities --test codec`

running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 28 tests
test a_different_holder_key_does_not_verify ... FAILED
test golden_apply_plan_decodes_to_typed_fields ... FAILED
test canonical_gate_rejects_non_canonical_bytes_with_a_valid_signature ... FAILED
test authority_epoch_above_i64_max_is_rejected_at_construction ... FAILED
test authority_epoch_boundaries_encode_minimally_and_round_trip ... FAILED
test authority_epoch_above_i64_max_is_rejected_at_decode ... FAILED
test golden_apply_plan_encodes_to_literal_bytes ... FAILED
test declared_byte_string_length_cannot_drive_an_allocation ... ok
test golden_fingerprint_digest ... FAILED
test golden_diagnostics_decodes_to_typed_fields ... FAILED
test golden_diagnostics_encodes_to_literal_bytes ... FAILED
test fingerprint_is_not_the_raw_key_namespace ... FAILED
test golden_token_identity_digest ... FAILED
test golden_fingerprint_preimage_is_algorithm_committing_cbor ... FAILED
test only_the_exact_ed25519_tag_resolves ... FAILED
test malformed_encodings_are_rejected ... ok
test bad_signatures_are_rejected ... FAILED
test oversize_input_is_rejected_by_the_pre_parse_ceiling ... FAILED
test raw_token_size_bounds ... FAILED
test token_identity_covers_every_raw_byte ... FAILED
test mutating_any_signed_byte_breaks_the_proof ... FAILED
test token_identity_is_not_the_undomained_raw_namespace ... FAILED
test presenting_a_different_token_is_rejected ... FAILED
[... remaining per-test lines elided; full failure list reproduced below ...]

---- weak_holder_keys_are_rejected stdout ----
thread 'weak_holder_keys_are_rejected' panicked at crates/mjolnir-admin-capabilities/tests/codec.rs:345:50:
called `Result::unwrap()` on an `Err` value: Malformed("token identity not implemented")

failures:
    a_different_holder_key_does_not_verify
    authority_epoch_above_i64_max_is_rejected_at_construction
    authority_epoch_above_i64_max_is_rejected_at_decode
    authority_epoch_boundaries_encode_minimally_and_round_trip
    bad_signatures_are_rejected
    canonical_gate_rejects_non_canonical_bytes_with_a_valid_signature
    fingerprint_is_not_the_raw_key_namespace
    golden_apply_plan_decodes_to_typed_fields
    golden_apply_plan_encodes_to_literal_bytes
    golden_diagnostics_decodes_to_typed_fields
    golden_diagnostics_encodes_to_literal_bytes
    golden_fingerprint_digest
    golden_fingerprint_preimage_is_algorithm_committing_cbor
    golden_token_identity_digest
    mutating_any_signed_byte_breaks_the_proof
    mutating_any_signed_field_breaks_the_proof
    only_the_exact_ed25519_tag_resolves
    oversize_input_is_rejected_by_the_pre_parse_ceiling
    presenting_a_different_token_is_rejected
    raw_token_size_bounds
    token_identity_covers_every_raw_byte
    token_identity_is_not_the_undomained_raw_namespace
    undecodable_holder_keys_are_rejected
    valid_proof_positive_control
    valid_proof_positive_control_for_apply_plan
    weak_holder_keys_are_rejected

test result: FAILED. 2 passed; 26 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Reading of the red run

26 of 28 failed. The two that passed are the reason positive controls matter:

- `malformed_encodings_are_rejected`
- `declared_byte_string_length_cannot_drive_an_allocation`

Both are negative-only assertions, so a stub that rejects *everything*
satisfies them. They are not evidence of a working parser on their own. They
are meaningful only jointly with the golden-vector and positive-control tests
(`golden_*`, `valid_proof_positive_control*`,
`authority_epoch_boundaries_encode_minimally_and_round_trip`), all of which
failed here and must pass after implementation. This is recorded so that a
later green run is not mistaken for a reject-everything implementation.

## Golden-vector provenance

The literal hex in `tests/codec.rs` is **local**, marked
local-until-`ikp-6yz.2` reconciliation, and was derived independently of the
Rust implementation:

- CBOR byte strings were laid out by hand from
  `openspec/changes/add-mesh-admin-capabilities/codec-profile.md` §"Canonical
  request and proof" using a throwaway Python script (definite array head,
  minimal uint heads, `0x58 0x20` byte-string heads), not by printing this
  crate's encoder output.
- The two BLAKE3 digests (`GOLDEN_TOKEN_IDENTITY`, `GOLDEN_FINGERPRINT`) and
  the two negative-namespace digests (`RAW_TOKEN_NAMESPACE_DIGEST`,
  `RAW_KEY_NAMESPACE_DIGEST`) were computed by a standalone `blake3 = "1"`
  binary in `/tmp` over those hand-derived preimages, again not via this crate.

## Scope honesty

Nothing in this suite asserts authorization, issuer/rights binding, trusted
time, expiry, revocation, target-challenge freshness, or replay resistance.
This slice implements none of those, and altered-byte rejection must not be
read as any of them.
