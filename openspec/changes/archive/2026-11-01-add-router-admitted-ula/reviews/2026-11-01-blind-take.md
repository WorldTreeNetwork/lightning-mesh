1. Pin both renderers to identical foreign `/48 eq 128` in/out allows before their `fc00::/7` denies; refuse `ge`, `le`, or covering-route permission.
2. Preserve local-house export and allow only for the usable `br-mesh` router `/128` observed by `known_house_router`.
3. Treat the admitted projection as caller authority, with no guild-log reads or signature decisions inside the renderer or router.
4. Specify the projection grammar and validation boundary so malformed, non-ULA, non-/48, duplicate, own-house, and compiled-stock inputs cannot broaden admission.
5. Define whether a missing usable local house suppresses foreign allows, and prevent a projection for an old house from surviving a house change unnoticed.
6. Pin startup and periodic reads in both reconcilers, including missing/unreadable input and removal, so stale foreign permissions are replaced.
7. Keep foreign prefixes out of address assignment and local redistribution; an admitted prefix authorizes learned hosts without originating them.
8. Preserve gateway export, IPv4 link-local next hops, the `10.254` Iroh bind, and the existing `/128` via `fe80::H` peer selection.
9. Verify both renderers with house plus foreign hosts, rejected covering prefixes, unrelated stock hosts, empty input, withdrawal, and unchanged-input stability.
10. Tradeoff: a trusted projection keeps routing independent of signing, but its house binding, replacement semantics, and successful Babel reload must carry admission freshness.
