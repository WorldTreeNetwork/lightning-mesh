# House ULA campaign — morning questions

Stop: every remaining in-scope node is blocked on question 1.

`bny6.3` `add-admin-app-tun` is accepted, implemented, and folded. `bny6.4` `add-admitted-prefix-route` is implemented from the accepted permit: the foreign `/48` is a device route on the open app TUN only while the caller presents both admit ops, and omitting either op removes it. That is not a new question. `bny6.6`, `bny6.7`, `bny6.5`, and `bny6.8` stay open until question 1 is answered.

Recorded steer is not re-asked. Pairwise countersign, the per-house `/48`, dual-assign, and trailing the `10.254` dial pin are unchanged.

## Spike verdicts

- `mjolnir-mesh-bny6.6` `house-ula-forward`: **go**. Babeld 1.14 installed the peer `/128` (`fd81:2b22:8aca::2 via fe80::… proto babel`), pinged it, left a stock prefix unrouted, and left a random address in the same `/64` unreachable. `eq128_installed=1`. The tunnel was a veth, not `mjolnir0`. IPv6 `eq` does not take babeld 1.13's IPv4 `+96` rewrite. This go does not measure the overlay demux.
- `mjolnir-mesh-bny6.3` `admin-tun-encap`: **go** after a discriminating rerun. The first same-netns ping did not prove seam traversal. The rerun derived `fd81:2b22:8aca:1:bd5e:fa7a:5460:697`, put each TUN in its own netns, lost the ping while the seam was off (`seam_off_ping_ok=false`), and with the seam on recorded the echo request and reply on the conn labeled with the peer node id (`echo_request_on_peer=2`, `echo_reply_on_peer=2`). It did not bind `fd` as an Iroh candidate and does not unlock `bny6.8`.

## Questions

### 1. How does a house `/128` cross sites?

Decision: once Babel has a house host route, which forwarding path may `add-ula-host-forwarding` claim?

`OverlayRouter::next_hop` (`crates/mjolnir-mesh/src/bin/mjolnir-meshd.rs`) returns a next hop for IPv6 only when the destination is `fe80`. Any `fd` destination is `None`, and `overlay_reader` then drops the unicast. `fib_mirror` reads only `ip -4 route`. A kernel route on `mjolnir0` does not select an Iroh connection. The house-ula-forward spike never entered that code.

Known options:

1. Same-site only in this node. Babel on `br-mesh` exports and accepts the house `/128` with a link-local IPv4-style next hop left as link-local IPv6. Cross-site delivery through `mjolnir0` waits on a later node. `fd` stays out of the Iroh bind set. `10.254` stays the dial pin.
2. This node also teaches the existing overlay demux to resolve an IPv6 host route onto an already-open Iroh connection, and to drop that resolution when the route is withdrawn. `fd` stays out of the Iroh candidate set. `10.254` stays the dial pin. IPv4 next hops stay link-local.
3. Land neither until a new spike measures option 2. Option 1's renderer work would wait with it.

Blocked until this is answered: `mjolnir-mesh-bny6.6` `add-ula-host-forwarding`, and behind it `bny6.7` `add-ula-node-reference`, `bny6.5` `add-router-admitted-ula`, and `bny6.8` `remove-v4-backhaul-pin`. `bny6.3` and `bny6.4` do not wait on this question.

Astra's other two forwarding notes are not separate questions. The `ge`/`le` ban must keep the existing gateway `le 0` line, and "known" means the usable house `/128` the daemon already publishes. Those edits wait until option 1, 2, or 3 is chosen, because they would otherwise rewrite a contract this run is not landing.
