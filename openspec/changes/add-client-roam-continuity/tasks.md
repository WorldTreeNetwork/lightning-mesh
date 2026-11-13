# Tasks

## DHCP continuity

- [x] `crdt/lease.rs`: render the static roam config and the hosts file
- [x] `crdt/lease.rs`: lease-file parser and fresh-ACK delta, with tests
- [x] Sandbox check of the rendered config against real dnsmasq
- [x] meshd: lease-file ingest replaces the spool; hosts file + SIGHUP;
      restart dnsmasq only when the static config changes
- [x] Remove `/usr/libexec/mjolnir-dhcp-event`; clear the `dhcpscript` UCI
- [x] `proxy_delay=0` on the client bridge
- [x] `deploy/sim/e2e/dhcp-crdt.sh` reads the hosts file

## Roam assistance

- [x] `roam_assist.rs`: neighbour-report entry, merge, expiry, ubus payloads
- [x] Appended `GossipMessage` variant; old-node decode-skip covered
- [x] meshd: publish own reports, install peers', re-assert on a loop

## Gates

- [x] `cargo test -p mjolnir-mesh --features daemon`, `cargo build --workspace`
      (clippy: no errors; existing warnings elsewhere left alone)
- [ ] Sim `e2e/run.sh dhcp-crdt` green
- [ ] Fleet rollout through `update-fleet.sh`
- [ ] Metal walk with a phone on a call (`0pv`, `wvg.4`)

## Handoffs

- Shared gateway MAC: bead under `wvg`, needs the shared-copper decision.
- Channel plan: both indoor client APs share 2.4 GHz channel 6.
