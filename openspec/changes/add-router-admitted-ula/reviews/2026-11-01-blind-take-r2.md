1. Pin the change to foreign-house host-route admission in both Babel renders; admission alone must originate no route.
2. Require matching in/out `/48 eq 128 allow` lines before the `fc00::/7` denies, with no foreign redistribution or covering address.
3. Keep the local router export and house allow conditional on a usable `/128` on `br-mesh`, including tentative and DAD-failed rejection.
4. Derive own-house exclusion separately from the configured guild, so an unusable router address cannot turn the local house into a foreign allow.
5. Exclude the compiled legacy prefix through `ula_prefix()`; refuse a supposed stock-prefix denylist that cannot establish house provenance.
6. Keep signature and foreign-house authority with the publisher; a syntactically valid `fd` `/48` does not prove admission or identity.
7. Require canonical, deduplicated input and explicit empty, missing, malformed, and unreadable projection behavior, including house changes.
8. Separate a successful config write from successful service application; both loops must retry a failed removal restart even with unchanged disk content.
9. Preserve gateway exports, IPv4 next hops, the `10.254` Iroh bind, and exact `/128` overlay route selection and withdrawal.
10. Accept last-good input on read failure as an availability tradeoff only with explicit stale-authorization limits and removal/retry tests.
