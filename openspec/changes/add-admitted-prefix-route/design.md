# Design

## Route

`admitted_pairs` is unchanged. It still writes nothing. `foreign_prefixes` keeps only the other house's `/48` when this house is one side of a pair. `sync_admitted_routes` installs `prefix/48 dev <app-tun>` for each of those prefixes and deletes a prefix this session installed when it is absent from the current set. The `/48` is not assigned on the TUN. The peer `/128` route from `open_app_tun` stays the host route. A session that is not up installs nothing.

## What this node does not change

Admission stays pairwise. One signature still installs nothing. No expiry and no withdrawal kind. Babel is unchanged. `10.254` stays the dial pin.

## Unverified hypotheses

None. Spike skipped: the route is the permit pair plus the TUN that `add-admin-app-tun` already opens.
