# Design: add-sim-roam-keep-ip

Instrument contract. Steer: sim is not metal. Advise r1 (Sol 2026-09-18)
required an ordered baseline, a move of proto 158, LWW lease, sequenced
egress, and property-red false-greens.

## Fixture

- STA is an OpenWrt guest from the same sim image, STA-only UCI (no
  mesh point, no meshd). Same `sim-guest` marker.
- Nodes A and B already run meshd + client AP + 802.11s (sim.4/sim.5).
- Hop mechanism is explicit in the log: `wpa_cli roam <B-bssid>`
  (forced) and/or `vwifi-ctrl set` (RSSI fade). The log names which
  was used. Green is never “command issued.”

## Pre-hop baseline (required)

Record, in order, before any roam command:

1. STA MAC
2. vended IPv4
3. home node id / overlay (A)
4. LeaseBook LWW entry for that MAC (IP + HLC)
5. `iw station dump` on A contains the STA; on B it does not
6. usable `ip neigh` on A for the STA (REACHABLE/DELAY/PROBE, not FAILED)
7. proto 158 `/32` for that IP **absent** on A and on B
8. egress probe succeeding via A (see probe)

`guest_routes` inputs are association + usable neighbour. The harness
must see both before expecting B to install the `/32`.

## Green (all required)

After the hop command:

- Observed association **transition**: STA in B's `iw station dump`,
  **not** in A's. Command-without-transition is red.
- IPv4 unchanged from baseline.
- proto 158 `/32` **present on B only**, **absent on A**. Appearance
  on B while A still has it is red (both announcing).
- LeaseBook current LWW: same MAC → same IPv4 (not first-write-wins
  folklore; keep the latest ACK). Divergence is red.
- Egress resumes **through B**: continuous probe (see below) has a
  timestamped last-success-before, first-success-after, loss count,
  and observed egress node B (not A, not mgmt-only).
- Proxy-ARP: STA can ARP/reach home gateway `10.42.<A>.1` while on B.

## Probe

- ICMP or TCP to a sink beyond B (or B's LAN gw) at a fixed cadence
  (default 200 ms) with monotonic sequence numbers.
- Log: t_last_ok_before, t_first_ok_after, loss_count in the hole,
  observed_egress_node.
- A single ping before/after is not enough.

## Evidence schema (log JSON or tagged lines)

```
mechanism: wpa_cli-roam | vwifi-ctrl
sta_mac, ipv4
assoc_a_before, assoc_b_before, neigh_nud_a
assoc_a_after, assoc_b_after
route158_a_before, route158_b_before
route158_a_after, route158_b_after
lease_ip, lease_hlc (unchanged)
probe: cadence_ms, seq_last_ok_before, seq_first_ok_after, loss_count
egress_node_after: B
```

## Property-red

The harness **fails** (non-zero) when any of:

- roam command ran but association did not move A→B
- A still has proto 158 `/32` after hop
- LeaseBook MAC→IP changed
- probe never resumes, or resumes only via A/mgmt
- client L2 was stitched (`br-lan` shared across nodes)

These cases are checked in CI against the script (fixture or recorded
transcripts), not only on a live hop.

## Not green

- `vwifi-ctrl` / `wpa_cli roam` exit 0 alone
- `/32` on B while A still advertises it
- closing `5wc` / `wvg` / `sz9.1`
