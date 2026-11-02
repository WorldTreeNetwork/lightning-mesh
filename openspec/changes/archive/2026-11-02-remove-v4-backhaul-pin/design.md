# remove-v4-backhaul-pin

## Decision

The Iroh bind and dial address is the usable house router `/128` on `br-mesh`. The endpoint is not created until that address is usable. The builder calls `clear_ip_transports` and then binds only that `/128`, so the preset IPv4 socket on `0.0.0.0` is gone. `10.254` is not assigned. pt9 does not run. There is no all-interfaces fallback. Explicit `--bind` replaces the house bind, `0.0.0.0` is rejected, and a failed bind does not fall back. LAN, overlay, and internet modes share this rule. Losing, replacing, or recreating the house address closes the endpoint and drops connection keys. Rebind waits until the new address is usable.

The connection key is `router_ula` of the local house guild and the authenticated peer endpoint id. That endpoint id is the peer node id string. Babel does not carry the `/128`. babeld 1.13 `router-id` is a modified EUI-64 or a MAC-48, and it is not the key. A missing or malformed guild file yields no key.

The overlay link-local on `mjolnir0` is `fe80::/64` plus the same 8-byte interface id as the router `/128` (`blake3` of the node id, zero clamped to 1). It is not derived from `10.254`, and it is not reversed into a key. IPv4 and IPv6 Babel next hops stay that link-local. A Hello or IHU (UDP 6696) sourced from that link-local and received on the authenticated connection associates the `via` with the key. A Hello heard only on `br-mesh` does not key an Iroh connection. A route via that link-local selects the connection. The destination `/128` is not the key. An admitted foreign host is transited by that same-house peer. This change does not key a direct foreign-house Iroh peer by a foreign guild. Two live connections on one link-local, or two link-local sources on one connection, select nothing. Disconnect or route withdrawal drops the selection on the next poll. Withdrawal of a route does not by itself drop the connection. IPv4 packets stay IPv4 and select the neighbor that announced the prefix.

The house `/128` stays on `br-mesh` and is not assigned on `mjolnir0`, so a kernel route on `mjolnir0` may still show a link-local via. Babel on `br-mesh` is the source of the underlay host route to a peer `/128`. Overlay mode runs that Babel in addition to Babel on `mjolnir0`. The daemon does not install a static peer route. The `br-mesh` Babel is up before the first dial and does not wait for Iroh. The endpoint may exist before any peer route. A dial waits for the `br-mesh` host route, or for an enabled relay when the peer is off-link. Outer datagrams from the local `/128` to a peer `/128` use the `br-mesh` route only. A `mjolnir0` route to the same destination is not selected, whatever its metric. With relays disabled, a missing `br-mesh` route waits and retries, and those datagrams are not sent. Relay datagrams use an underlay path that is not `mjolnir0`. Losing the `br-mesh` route does not move outer UDP onto `mjolnir0`. Inner house-host packets still follow Babel on `mjolnir0`. Before overlay host routes exist, unicast to the associated overlay link-local returns on the connection that received the Hello or IHU. Multicast `ff02::1:6` goes to every established connection. The `br-mesh` next hop is that interface's link-local. It is not the overlay `fe80` and it is not the connection key.

The walked fleet column stays `10.254` in this change. SSH to that column will not reach a node after the address is gone. Reachability for operators is the `/128` from `add-ula-node-reference`. This change does not rewrite the inventory file.

## Measured

- id: `iroh-ula-dial`
- claim: current Iroh binds and dials the house ULA without a duplicate-address failure
- confidence: high
- verification: empirically tested
- synthesis path: none
- spike-artifact: `docs/spikes/iroh-ula-dial/finding.md`

Validated by spike `iroh-ula-dial` on 2026-11-01. Verdict go. The `/128` bound and dialed once each. The same builder still binds `0.0.0.0` because the spike did not call `clear_ip_transports`. The spike installed host routes by hand and used `nodad`. It does not prove cold start, the Hello association, or endpoint recovery. Those are specified above and are checked in the delta scenarios.
