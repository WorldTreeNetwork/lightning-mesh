# AP3000 Outdoor vs DWeb m3000 — config matching (next-agent handoff)

Written / re-verified **2026-08-30**. This pair is the event two-node island.
Do **not** re-PSK the client AP, mix open + PSK on the same SSID, or change
`MESH_ID` / backhaul channel unless asked. SSID stays `Lightning Mesh`.

Live-checked this session (hostapd `wpa=0`, UCI `encryption=none`, no
`wireless.*.key`, laptop scan SECURITY column empty on both BSSIDs):

| box | client SSID | encryption | password |
|-----|-------------|------------|----------|
| `ap3000-outdoor` | `Lightning Mesh` | **`none` (OPEN)** | **none** |
| `m3000` (DWeb field box) | `Lightning Mesh` | **`none` (OPEN)** | **none** |

`m3000` is the source of truth for **radio identity**. AP3000 was matched to
it (was `psk2` / `lightning!`; opened 2026-08-30). Remaining diffs below were
left alone on purpose.

## Reach (from this house)

Workstation is usually on **Origami Springs** (`10.0.0.0/24`) so the chat
survives. Overlay `10.254.x` is **not** reachable from Origami; jump via the
AP3000 WAN lease.

| name | board | overlay | LAN gw | WAN (house) | jump |
|------|-------|---------|--------|-------------|------|
| `ap3000-outdoor` | Cudy AP3000 Outdoor v1 | `10.254.166.226` | `10.42.166.1` | `10.0.0.239` | `ssh root@10.0.0.239` |
| `m3000` | Cudy M3000 v1 | `10.254.242.172` | `10.42.242.1` | no live WAN IPv4 | `ssh -o ProxyCommand='ssh -W %h:%p root@10.0.0.239' root@10.254.242.172` |

Dropbear keys: **`/etc/dropbear/authorized_keys`** (not `~/.ssh`).
Workstation: `~/.ssh/id_ed25519` (`dorje@Framework5070`). `m3000` has **no**
empty root password — key required. AP3000 currently accepts that key (and
still has an empty root password from flash).

```sh
# IPv6 LL on AP3000 br-lan (zone = wifi iface, e.g. wlp191s0):
ssh root@fe80::82af:caff:fef2:20f5%wlp191s0
```

Do **not** run `mjolnir-meshd id` / `diag` on a live node — those subcommands
start a second daemon. Use `uci get mjolnir.meshd.name` and
`deploy/openwrt/fleet-nodes.conf`.

BSSIDs (scan identifiers):

| role | AP3000 | m3000 |
|------|--------|-------|
| client AP `Lightning Mesh` (2.4 ch 6) | `80:AF:CA:F2:20:F5` | `80:AF:CA:E7:BD:00` |
| 802.11s `mjolnir-mesh` (5 GHz ch 36) | `82:af:ca:f2:20:f6` | `82:af:ca:e7:bd:01` |

Mesh plink **ESTAB** ~−40 dBm when they are in the same room.

---

## Matched (keep aligned on every new node)

These knobs are the same on both boxes. A third node must copy **this**
column, not `setup-wireless.sh` defaults.

| knob | live value | UCI |
|------|------------|-----|
| Client SSID | `Lightning Mesh` | `wireless.clientap.ssid` |
| Client AP encryption | **`none` (OPEN, no password)** | `wireless.clientap.encryption='none'`; **no** `.key` |
| Disabled co-located AP | same SSID, **also `none`**, `disabled=1` | `wireless.clientap2g` |
| 802.11s mesh id | `mjolnir-mesh` | `wireless.meshbh.mesh_id` |
| Mesh encryption | `none` | `wireless.meshbh.encryption` |
| Mesh forwarding | on | `wireless.meshbh.mesh_fwding='1'` |
| Backhaul | 5 GHz **ch 36 HE80** on radio1 | `wireless.radio1` + `wireless.meshbh.device='radio1'` |
| Client AP | 2.4 GHz **ch 6 HE20** on radio0 | `wireless.radio0` + `wireless.clientap.device='radio0'` |
| Default OpenWrt SSIDs | disabled | `wireless.default_radio{0,1}.disabled='1'` |
| meshd mode | `lan`, `lan_tunnels=0` | `mjolnir.meshd` |
| backhaul / client ifaces | `br-mesh` / `br-lan` | `mjolnir.meshd.backhaul_iface` / `client_iface` |
| Overlay | `10.254.<derived>/16` on `br-mesh` | from node id, recorded in `fleet-nodes.conf` |
| Client LAN | `10.42.<octet>.1/24` on `br-lan` | AP3000 `10.42.166.1`, m3000 `10.42.242.1` |
| `CLIENT_AP_2G` | disabled | mt76 mesh+AP on same radio is `oaq` — do not enable |

### Fleet env the next install **must** pass

`setup-wireless.sh` defaults (as of 2026-08-30) match this pair: open
`Lightning Mesh`, `COUNTRY=US`, open 802.11s `mjolnir-mesh`. Still pass the
env file so a future default drift cannot re-PSK the fleet:

```
deploy/openwrt/install-node.sh --wireless deploy/openwrt/fleet-secrets/wireless.env root@<node>
```

Checked-in template: `fleet-secrets/wireless.env.example`. Live gitignored
file (`fleet-secrets/wireless.env`) is:

```
MESH_ID='mjolnir-mesh'
MESH_KEY=''
CLIENT_SSID='Lightning Mesh'
CLIENT_ENC='none'
CLIENT_KEY=''
CLIENT_AP_2G_ENC='none'
FT_KEY=''
COUNTRY='US'
BACKHAUL_BAND='5g'
BACKHAUL_CHANNEL_5G='36'
CLIENT_CHANNEL_2G='6'
```

`COUNTRY='US'` matches AP3000 only (m3000 is `DE`). Do not fleet-roll
`--wireless` onto m3000 unless you intend to change its country.

After any client-encryption change, clients must **forget** saved
`Lightning Mesh` WPA profiles (open vs PSK is a different network). Laptop
last confirmed on Origami Springs; the old WPA2 NM profile was deleted.

Verify open after apply:

```sh
uci get wireless.clientap.encryption    # none
uci get wireless.clientap.key           # must fail / empty
grep -E 'ssid=|wpa=' /var/run/hostapd-phy0.conf   # ssid=Lightning Mesh, wpa=0
# from the laptop:
nmcli -f SSID,SECURITY,BSSID,CHAN device wifi list | grep -i Lightning
# SECURITY column empty on BOTH BSSIDs
```

To open a node that came up PSK (what we did on AP3000):

```sh
uci set wireless.clientap.encryption='none'
uci -q delete wireless.clientap.key
uci set wireless.clientap2g.encryption='none'
uci -q delete wireless.clientap2g.key
uci commit wireless
wifi reload
```

---

## Remaining differences (do **not** “fix” unless asked)

Full live UCI/runtime compare, 2026-08-30.

| | `ap3000-outdoor` | `m3000` (DWeb, source of radio truth) |
|---|---|---|
| OpenWrt | **25.12.5** r33051, kernel **6.12.94** (sysupgrade from 24.10.8 on 2026-08-30, config kept) | **25.12.4** r32933, kernel **6.12.87** (image 2026-05-13) |
| `mjolnir-meshd` | `74dd833-dirty` (local `roam.rs` so it would compile) sha256 `b4afc228…` 2026-08-30 | older binary **2026-08-08** sha256 `c8bce2bb…` |
| hello.mesh | UCI `hello.enabled=1`, **no binary**, :80 closed, DHCP 114 **cleared** | UCI enabled, **`/usr/bin/mjolnir-hello` + `S97mjolnir-hello`**, DHCP `114,http://hello.mesh/api/captive-portal`; LuCI on **:8080** |
| Country | `US` / `US` | `DE` / `DE` (ch 6 / 36 still legal) |
| Ethernet | single `eth0` = WAN DHCP `10.0.0.239/24` via `10.0.0.1`; LAN is wifi-only (no port on `br-lan`) | `eth0` WAN (no live IPv4), `br-lan` ports=`eth1` |
| Extra LAN IP | **none** (do not add `192.168.1.1`) | `br-lan` also has **`192.168.1.1/24`** beside `10.42.242.1/24` |
| Default route | WAN `eth0` — babel **exports `0.0.0.0/0` metric 128** (house internet for the mesh) | via AP3000 overlay `10.254.166.226` — **does not** export default |
| `mjolnir.meshd.gateway` | `auto` | unset |
| WAN firewall | `forward=REJECT`, extra **Allow-SSH-WAN** | `forward=DROP`, no SSH-on-WAN rule |
| UCI peers | four fleet ids **including m3000** `7faf041c…` | three fleet ids — **no AP3000 id** `0d115994…` yet |
| SSH | empty root password + key | key required (no empty password) |
| Packages | apk: `babeld`, `kmod-tun`, `iperf3`; currently `wpad-basic-mbedtls` (open 802.11s; wpad-mesh swap health-gated on first apply) | 25.12 apk; wpad-mesh is running |
| UCI `system.@system[0].hostname` | still `OpenWrt` | still `OpenWrt` |
| Kernel / meshd name | `ap3000-outdoor` | `m3000` |
| Node id / overlay | `0d115994…2a4b` → `10.254.166.226` | `7faf041c…d213` → `10.254.242.172` |
| 2.4 GHz txpower | 28 dBm | 20 dBm (country / board) |
| `wireless.clientap.isolate` | `0` (explicit) | unset |
| `network.lan` syntax | CIDR `10.42.166.1/24` only (25.12 re-added `192.168.1.1/24` on first boot; **stripped again**) | CIDR list (25.12) including leftover `192.168.1.1/24` |
| `network.loopback.ipaddr` | `127.0.0.1` + netmask | `127.0.0.1/8` |
| ULA | `fd28:3ec4:86c1::/48` | `fdc3:211e:1ad2::/48` |

`fleet-nodes.conf` already has both lines. AP3000 WAN lease `10.0.0.239` is
house-LAN specific; if the AP is moved to another upstream, DHCP will change
and Origami-side SSH must follow the new lease (IPv6 LL on br-lan still
works from a client associated to `Lightning Mesh`).

---

## Do not blindly unify

- Do **not** add `192.168.1.1/24` onto AP3000 `br-lan`. Dual LAN IPs broke
  client → gateway (ARP reachable, IPv4 ping blackhole).
- Do **not** put a PSK on `Lightning Mesh` while the other AP is open — two
  APs, same SSID, different security = clients fail to join.
- Do **not** enable `wireless.clientap2g` (mesh+AP on the same mt76 radio).
- Do **not** change mesh id / channel; they already ESTAB.
- Do **not** `ip route replace default via 10.42.166.1` from the laptop while
  Origami is the working uplink — that killed this chat. Join Lightning Mesh
  only when you intend to use its NAT, and wait for a `10.42.166.x` DHCP
  lease before moving the default route.
- AP3000 `gateway=auto` is why this house NATs mesh clients. Leave it.

---

## Installing the next node to **match this pair**

1. Flash OpenWrt (AP3000 Outdoor v1: stock Cudy web → Cudy-signed
   intermediate in `vendor-firmware/ap3000-outdoor-v1/` → official
   sysupgrade). Indoor AP3000 V1/V1.1 uses `vendor-firmware/ap3000-v1/`,
   not the Outdoor bin. This outdoor box was brought up on 24.10.8, then
   **sysupgrade (keep config)** to **25.12.5** on 2026-08-30.
   Add `/etc/mjolnir/` to `/etc/sysupgrade.conf` first or the node id is
   regenerated. 25.12 first boot re-added `192.168.1.1/24` on `br-lan` —
   strip it. Stock Cudy management IP was `192.168.10.254`, not `.1`.
2. Put the box on ethernet. Install:

   ```sh
   deploy/openwrt/install-node.sh \
     --wireless deploy/openwrt/fleet-secrets/wireless.env \
     root@<fresh-ip>
   ```

   If `fleet-secrets/wireless.env` is missing, copy the `.example` — it is
   already `CLIENT_ENC=none`. **Never** run `setup-wireless.sh` without that
   env on this event pair.
3. `uci set mjolnir.meshd.name='…'` and `list peer` **every** other node id
   (full mesh, not a chain). Add this node's id to the existing boxes'
   peer lists (m3000 is still missing AP3000's id).
4. Append a line to `deploy/openwrt/fleet-nodes.conf`
   (`name|10.254.x.y|node_id|model|notes`). Overlay address is derived from
   the node id; confirm with `ip -4 addr show br-mesh`, not `mjolnir-meshd id`.
5. Drop the workstation pubkey into `/etc/dropbear/authorized_keys`.
6. Confirm scan: `Lightning Mesh` SECURITY empty on the new BSSID **and** on
   both existing BSSIDs. Confirm 802.11s ESTAB to a peer on ch 36.
7. Do **not** add `192.168.1.1` on the new LAN. Do **not** enable hello /
   DHCP option 114 unless the hello binary is actually installed.

AP3000 flash / sysupgrade / `roam.rs` compile notes from the bring-up live in
the session history; the radio matching that matters for the next box is
entirely this file plus `fleet-secrets/wireless.env`.
