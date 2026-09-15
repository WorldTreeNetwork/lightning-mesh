# aarch64 musl size delta (ncy.1 / add-mini-app-contract)

Measured 2026-09-15 with `SKIP_WEB=1 deploy/openwrt/build-hello.sh`
(`messense/rust-musl-cross:aarch64-musl`, `--release --locked`, stripped
static ELF). Dirty `crates/mjolnir-hello/static/index.html` was copied
aside and restored; embed used committed static.

| | commit | bytes | MiB |
|---|---|---:|---:|
| before (no ureq/rustls) | `5654ce5` (`503cd17^`) | 1 316 480 | 1.26 |
| after (HEAD + fetcher) | `4606896` tree at measure | 2 889 480 | 2.76 |
| **delta** | | **+1 573 000** | **+1.50 MiB (+119.5%)** |

The delta is **not TLS-only**: the before embed is the older `static/`
at `5654ce5`. It still includes `ureq` 2.12 + `rustls` 0.23 (ring) on
the after binary. Design said if the TLS stack is unacceptable, drop to
plain HTTP via a change amendment. This measurement does not by itself
authorize that drop.

No live install. Binary is gitignored at `deploy/openwrt/mjolnir-hello-aarch64`.
