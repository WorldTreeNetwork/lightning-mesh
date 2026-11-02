# Tasks

- [x] Accept an admitted `/48` slice on both Babel renders and emit `eq 128` allows before the `fc00::/7` denies
- [x] Read `/var/lib/mjolnir/admitted-prefixes` on each reconcile; a missing file is an empty list
- [x] Count only an `fd00::/8` `/48`. Skip comments, `ula_prefix()` (`fd53:6213:4797::/48`), and the guild-file house `/48` even when the `/128` is unusable. Do not denylist `fd12:3456:789a::/48`
- [x] Do not redistribute or assign the foreign prefix on `br-mesh`, `mjolnir0`, or `br-lan`
- [x] Lock the fixture: house `/128` and `fd8f:78a9:2caa::/48 eq 128` are in both renders; `fd12:3456:789a::/48` and the rest of `fc00::/7` are denied; gateway `le 0` is unchanged; new lines have no `ge` or `le`
- [x] Lock the demux regression: an admitted `/128` via `fe80::2` selects `10.254.0.2`, and `fd` stays out of the Iroh bind set
- [x] Keep a pending Babel restart in both reconcilers until start or restart succeeds, and retry it on the next paced pass when the rendered text is unchanged
- [x] Treat a missing projection as an empty list and any other read error as "keep the last successful projection"

## Owed from advise (2026-11-01, astra-arch-review)

- [x] R1: Pin own-house exclusion independently of usable `house_router`; specify and cover configured-house/unusable-address, missing or malformed guild, and guild-change cases with own plus foreign projection entries in both renders (see reviews/2026-11-01-advise.md)
- [x] R2: Identify the exact compiled-prefix exclusion and distinguish it from stock OpenWrt prefixes; pin trusted-publisher versus reader responsibilities, allowed IPv6 range, and supplied-stock/direct-renderer negative fixtures consistently in design and delta
- [x] R3: Specify projection removal and unreadable-file behavior, plus successful Babel application/retry semantics in both reconcilers; require a failed start/restart followed by unchanged-input retry test so persisted config cannot masquerade as applied policy
- [x] Fresh advise after preparation
- [x] Obtain fresh other-family advise after reconciling R1–R3 and before act
