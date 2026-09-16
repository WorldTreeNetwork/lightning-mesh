# steer add-https-aliases

**When.** 2026-09-15
**Depth.** standard (activation; remaining forks decide-for-me)

## Decided

- **Activate.** Duke named `add-https-aliases` / `mjolnir-mesh-b6j.1`. Banner ACTIVE BUILD.
- **Label.** 80-bit blake3, RFC 4648 base32 lowercase no padding, 16 chars. (decide-for-me)
- **Resolution records.** In this change, not `ai0.9` (admin records). Matches b6j steer split. (decide-for-me)
- **Router TLS.** tiny_http + rustls ring; measure on aarch64; terminator is fallback only. (decide-for-me)
- **Default adapter.** WTN best-effort for `mesh.worldtree.network`; not a mesh availability dependency; BYO zone = own adapter. (decide-for-me)

## Skipped

- Live ACME issuance against production Let's Encrypt — not this activation
- Hard custody / signer (`ai0`)
- Mini-app secure embedding (needs hello HTTPS origin first)

## Feeds change

Pins are in `design.md`. Independent non-Claude advise next; do not act until accept.
