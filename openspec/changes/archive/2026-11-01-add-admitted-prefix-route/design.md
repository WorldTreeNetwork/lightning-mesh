# Design

## Route

`admitted_pairs` is unchanged. It still writes nothing. `prefixes_for_house` keeps only the other house's `/48` when the house that opened the session is one side of a pair, and it drops that house's own `/48`. `sync_admitted_routes` takes only the presented pairs. The house id is the one stored when `open_app_tun` opened the session; a later call cannot supply a different guild. It installs `prefix/48 dev <app-tun>` for each kept prefix and deletes a prefix this session installed when it is absent from the current set. A failed delete, or a failed observation of whether the route is gone, keeps that prefix owned and the call returns an error so the next sync retries. A replace that succeeds while the following observation fails also keeps that prefix owned. Confirmed absence drops it. The `/48` is not assigned on the TUN. The peer `/128` route from `open_app_tun` stays the host route. A session that is not up installs nothing.

## What this node does not change

Admission stays pairwise. One signature still installs nothing. No expiry and no withdrawal kind. Babel is unchanged. `10.254` stays the dial pin.

## Unverified hypotheses

None. Spike skipped: the route is the permit pair plus the TUN that `add-admin-app-tun` already opens.
