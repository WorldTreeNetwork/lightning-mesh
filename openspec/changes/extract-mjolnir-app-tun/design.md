# Design

## Dependency

`crates/mjolnir-app-tun` is a workspace member. It owns the house and app address functions, `DatagramConn`, `spawn_encap_pair`, and the Linux session. `mjolnir-mesh` depends on it and re-exports what the daemon already calls. Admin depends on it and deletes the session from `app_tun.rs`. The new crate does not depend on `mjolnir-mesh` and does not enable feature `daemon`.

`router_ula` stays callable by the daemon. The guild-id parser the address functions need moves with them.

## Session

The moved session matches the landed Admin runner: one TUN, MTU 1280, name `mja{n}`, local `/128` with `nodad`, one peer `/128`, encap supervision, and drop cleanup. A failed route delete or a failed observation keeps the prefix owned. Non-Linux reports the computed address with up false and installs nothing.

Foreign `/48` sync takes the guild id stored at open and a list of 6-byte prefixes. Admin turns `admitted_pairs` into that list. The crate does not read a guild log.

## What this node does not change

The guild surface still blanks a nested access guild and still does not open a TUN or stage the client SSID. `fd` is not an Iroh candidate. `10.254` stays the dial pin. Babel is unchanged.

## Unverified hypotheses

None. Spike skipped. `docs/spikes/admin-tun-encap/finding.md` is go, and the route retry is already tested on the session this change moves.
