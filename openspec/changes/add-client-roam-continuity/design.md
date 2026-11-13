# Design: client roam continuity

## Shape kept

One routed `/24` per node. Client L2 is never bridged across nodes. A roamed
phone keeps its address; the node it is on holds a mobility `/32` (proto 158)
that babel carries mesh-wide (`sz9`). This change repairs the DHCP and radio
edges of that design. It does not replace it.

## DHCP: leaderless, by reservation

Address *allocation* stays local: a node only picks new addresses from its
own `/24`, so two nodes can never hand out the same address. Address
*confirmation* is mesh-wide: any node confirms a `MAC → IP` pair found in the
gossiped lease book. That split is why no leader is needed.

dnsmasq config, proven in a network-namespace sandbox on 2.93 (2026-11-13):

Static part, `mjolnir-roam.conf` in dnsmasq's `conf-dir`. Written when the
node's own subnet is known. Changing it needs a dnsmasq restart.

```
dhcp-hostsfile=<conf-dir>/.mjolnir-roam.hosts
# for every x in 1..=254 except the node's own third octet:
shared-network=br-lan,10.42.x.0
dhcp-range=set:gx,10.42.x.0,static,255.255.255.0,12h
dhcp-option=tag:gx,option:router,10.42.x.1
dhcp-option=tag:gx,option:dns-server,10.42.x.1
```

Dynamic part, `.mjolnir-roam.hosts`, one `mac,ip` line per lease-book entry
in `10.42.0.0/16`. Own-subnet entries are listed too: a phone that keeps
renewing on another node outlives its lease at home, and the reservation
stops the home node handing that address to someone else. dnsmasq re-reads it on SIGHUP. The leading dot
makes `conf-dir` skip it as a config file. It lives in `conf-dir` because
that directory is already mounted into the dnsmasq jail.

Sandbox results:

| Case | Today | With this config |
|---|---|---|
| REQUEST for a reserved foreign address | NAK, 20 ms | ACK same address, 40 ms |
| DISCOVER from a MAC with a foreign reservation | own-subnet offer, 3.0 s | foreign address offered, 17 ms |
| New reservation added, SIGHUP only | — | honoured |
| DISCOVER from an unknown MAC | own-subnet offer | own-subnet offer (unchanged) |
| REQUEST for a foreign address with no reservation | NAK | NAK |

The last row is the accepted limit: a node that has not yet heard the lease
renumbers the phone. Gossip publishes within one poll of the original ACK and
the lease book is persisted, so the window is a node that was offline for the
phone's whole stay.

The gateway and DNS a roamed phone is told stay `10.42.<home>.1`. Proxy ARP
answers for that address on the node the phone is on.

## Lease ingest: read the lease file

meshd polls `/tmp/dhcp.leases` (`<expiry> <mac> <ip> <hostname> <client-id>`).
An entry is published when it is new to the book, or when its expiry has
advanced since the last poll (a fresh ACK). On the first poll after start, an
entry already in the book with the same address is not re-stamped: a restart
must not make an old lease look newer than a later one elsewhere.

The hook is removed and the `dhcpscript` UCI option is cleared. A hook that
OpenWrt sources inside a jail is the wrong tool for a daemon that can read
the file.

LWW on HLC stays: the latest ACK anywhere is the address every node confirms.

## Roam assistance

Each node reads its own neighbour report per client AP
(`ubus call hostapd.<ap> rrm_nr_get_own` → `[bssid, ssid, nr_hex]`) and
gossips it keyed by BSSID. Each node installs every peer report whose SSID
matches the AP (`rrm_nr_set`) and turns on neighbour report, beacon report
and BSS transition (`bss_mgmt_enable`). Both calls are runtime ubus calls:
no `wifi reload`, no client drop. A reconcile loop re-asserts them, since a
hostapd restart forgets them.

Entries expire if their owner stops refreshing them, so a dead node leaves
the neighbour list.

This is advice to the phone. It shortens scans and helps a phone leave a weak
AP sooner. It does not steer or kick clients; no `usteer`/`dawn`.

Works on an open SSID. 802.11r stays off there: there is no key handshake to
shorten.

## Decision deferred: shared gateway MAC

Phones check their gateway after a hop by unicast ARP to the gateway's
*remembered MAC* (RFC 4436; iOS does this). On a new node that MAC is absent,
the check fails, and the phone falls back to DHCP plus a fresh ARP: about a
second with this change, instead of a renumber.

Making it zero needs every node to answer on the same gateway MAC. The same
gateway *IP* is not needed, because proxy ARP already answers for every
`10.42.x.1`. This is the standard anycast-gateway technique (VRRP virtual
MAC, EVPN distributed gateway). It is not confusing on Wi-Fi: client L2 is
per node, so the MAC never appears twice in one broadcast domain.

It is deferred because `br-lan` also holds the wired LAN ports. Two nodes
cabled to one switch would put the same MAC on one segment. That is the
failure the fleet-identical `192.168.1.1` alias already caused (AP3000,
2026-09-15). It needs either a Wi-Fi-only scope or a shared-copper detector
first. Tracked as a bead under `wvg`.
