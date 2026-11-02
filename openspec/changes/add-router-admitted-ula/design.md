# add-router-admitted-ula

## Decision

The landed house allow stays. An admitted foreign `/48` is a second `eq 128` allow on both renders. The foreign prefix is not installed on any interface.

`foreign-prefix-permit` decides which pair counts, and it does not write Babel. This node does not move that predicate into the router. The daemon reads a projection the caller already filtered.

## Seam

`BabelConfigInputs.admitted` is a slice of IPv6 `/48` network addresses. `render_overlay_babeld_conf` takes the same slice. Both reconcilers pass `read_admitted_prefixes()` every pass, the same way they pass `known_house_router`.

For each address that counts, after the house lines when a house router is known, and in their place when it is not:

```
in ip {prefix}/48 eq 128 allow
out ip {prefix}/48 eq 128 allow
```

Then the existing `in ip fc00::/7 deny` and `out ip fc00::/7 deny`. First match wins, so these lines must stay above those denies. The new lines do not use `ge` or `le`. There is no `redistribute local` line for a foreign address. The gateway line `redistribute ip 0.0.0.0/0 le 0 metric 128` stays when the gateway is on and stays absent when it is off.

## Projection file

Path: `/var/lib/mjolnir/admitted-prefixes`.

The publisher is trusted routing input. It selects pairs for one house and replaces the whole file when that house changes. The reader cannot see that relationship from prefix bytes. It does not verify signatures, and it does not admit a prefix.

Reader rules, applied in order:

- Missing file (`ENOENT`): empty list. That is a removal.
- Unreadable file (any other I/O error): keep the last successful projection. If this process has never read one, do not render a removal; skip the projection update for this pass.
- Blank lines and `#` comments are ignored.
- A line counts only when it is an `fd00::/8` network address with the last 80 bits zero. A `/48` suffix is optional. Anything outside `fd00::/8`, including `fc00::/7` addresses that are not `fd`, does not count.
- Skip `ula_prefix()` from `crates/mjolnir-mesh/src/tun/link.rs`. For this build that is `fd53:6213:4797::/48`, the compiled legacy mesh prefix. That skip is not a general stock-OpenWrt denylist.
- Skip the house `/48` parsed from `/etc/mjolnir/house-guild-id` when that file is 64 hex digits. This uses `house_ula_prefix`, not `known_house_router`. A tentative, DAD-failed, missing, or wrong-interface house `/128` still excludes that prefix. `house_router == None` does not mean "every remaining line is foreign."
- A missing or malformed guild file excludes no house prefix. The publisher must not leave this house's prefix in the file across that transition.
- A guild change excludes the new house `/48` on the next reconcile. A line that was this house and is now the old prefix counts, until the publisher replaces the file.
- There is no reader rule that rejects `fd12:3456:789a::/48`. That prefix is forbidden publisher input. If a test passes it straight to the renderer, the allow is emitted. The happy-path fixture simply does not contain it, so both renders lack that string.

Duplicate lines collapse to one allow. Order is lexicographic on the address text, so an equivalent set does not churn the render.

The file is not authority. A stale line keeps the allow until the publisher removes it or the file disappears. There is no withdrawal op and no expiry, matching the permit.

## Apply, not just write

Both reconcilers already sleep 5 seconds between passes. The LAN loop also waits out `BABEL_SETTLE` before writing a new render.

A changed projection is not applied until `babeld` start or restart succeeds. `last_rendered` and a `changed == false` write must not swallow a failed restart. Both loops keep a pending restart until that call succeeds, and the next pass retries it even when the rendered text and the on-disk file are unchanged. The retry uses the existing pass pacing. It does not spin.

The owed test is: foreign prefix B is allowed, then B is omitted or the file is missing, the config is written, restart fails, and the next pass with those same inputs retries and applies the deny.

## Demux

`ula_host` already selects any IPv6 `/128` whose next hop is overlay `fe80::H` onto the open `10.254` peer that derives `H`. This node does not add a second selector and does not bind `fd`. A regression test uses an admitted destination, `fd8f:78a9:2caa::2/128 via fe80::2`, and expects `10.254.0.2`.

## Fixture

- House A router `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`, prefix `fd81:2b22:8aca::/48`.
- Admitted foreign prefix `fd8f:78a9:2caa::/48`.
- Stock negative `fd12:3456:789a::/48` is absent from both renders.
- `10.254.0.2` pairs with `fe80::2`.

## What does not change

IPv4 next hops stay link-local. The legacy `/64` stays unexported. `10.254` stays the Iroh bind. The IPv4 route mirror stays `ip -4 route`. Assigning the house `/128` still does not make the rest of that `/48` on-link.
