// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

const HTTPS_LABEL_BYTES: usize = 10;
const HTTPS_LABEL_LEN: usize = 16;
const BASE32_ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Derive the opaque HTTPS DNS label for an Ed25519 owner public key.
///
/// Production callers pass the complete 32-byte public key. Keeping the
/// primitive byte-oriented also lets cross-language tests cover BLAKE3's
/// standard empty-input vector directly.
pub fn https_label(owner_pubkey: &[u8]) -> String {
    let digest = blake3::hash(owner_pubkey);
    base32_lower_no_pad(&digest.as_bytes()[..HTTPS_LABEL_BYTES])
}

fn base32_lower_no_pad(bytes: &[u8]) -> String {
    let mut label = String::with_capacity(HTTPS_LABEL_LEN);
    let mut accumulator = 0_u16;
    let mut bits = 0_u8;

    for &byte in bytes {
        accumulator = (accumulator << 8) | u16::from(byte);
        bits += 8;

        while bits >= 5 {
            bits -= 5;
            let index = usize::from((accumulator >> bits) & 0x1f);
            label.push(char::from(BASE32_ALPHABET[index]));

            let mask = if bits == 0 { 0 } else { (1_u16 << bits) - 1 };
            accumulator &= mask;
        }
    }

    debug_assert_eq!(bits, 0, "the 80-bit prefix must align to base32");
    debug_assert_eq!(label.len(), HTTPS_LABEL_LEN);
    label
}

#[cfg(test)]
mod tests {
    use super::{BASE32_ALPHABET, HTTPS_LABEL_LEN, https_label};
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct LabelFixture {
        name: String,
        owner_pubkey_hex: String,
        label: String,
    }

    fn fixtures() -> Vec<LabelFixture> {
        serde_json::from_str(include_str!(
            "../../../hello-mesh-web/src/lib/https/fixtures/labels.json"
        ))
        .expect("shared HTTPS label fixtures must be valid JSON")
    }

    fn decode_hex(hex: &str) -> Vec<u8> {
        assert!(
            hex.len().is_multiple_of(2),
            "fixture hex must have byte pairs"
        );
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("fixture hex must be UTF-8");
                u8::from_str_radix(pair, 16).expect("fixture key must be hexadecimal")
            })
            .collect()
    }

    #[test]
    fn https_label_matches_shared_fixtures() {
        for fixture in fixtures() {
            let actual = https_label(&decode_hex(&fixture.owner_pubkey_hex));
            assert_eq!(actual, fixture.label, "fixture: {}", fixture.name);
            assert_eq!(actual.len(), HTTPS_LABEL_LEN, "fixture: {}", fixture.name);
            assert!(
                actual.bytes().all(|byte| BASE32_ALPHABET.contains(&byte)),
                "fixture: {}",
                fixture.name
            );
        }
    }
}
