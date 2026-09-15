# Household hardware inventory

Evidence-only inventory for planning an upstream receiver and a safe wired
management arrangement. Every line is either **observed** (dated, with the repo
file or bead it came from), **documented default** (what a repo file specifies,
which is not the same as a live reading), or **unknown**. Advertised
driver/role combinations are not treated as qualified concurrent operation.

Documentation node (`mjolnir-mesh-ai0.4`). No SSH, scans, deploys or config
changes were made to produce it. Live qualification belongs to
`mjolnir-mesh-z3th` (reboot / radio-reapply survival of the pilot uplink) and
`mjolnir-mesh-lpv` (per-SKU profile + failure-recovery qualification).

---

## 1. Current pilot — three nodes, observed 2026-09-14

Source: `bd show mjolnir-mesh-rple` (closed 2026-09-15; notes dated
2026-09-14 America/Los_Angeles). Models are as recorded in
`deploy/openwrt/fleet-nodes.conf`; where `fleet.yml` (2026-08-31) disagrees it
is noted — neither is a unit-label reading.

| Node | Model (per fleet-nodes.conf) | Overlay | Role observed 2026-09-14 |
|---|---|---|---|
| `m3000-b` | Cudy M3000 **v1/v2** (not narrowed to a revision) | `10.254.12.214` | **Gateway.** `phy0-sta0` station associated to the *Symbio* 2.4 GHz network, DHCP lease `10.43.1.159/24`, gw `10.43.1.254`. `radio0` clientap **disabled**. `radio1` 802.11s mesh ch36 HE80 retained. Routed WWAN DHCP metric 20 added to the existing masqueraded WAN zone; `gateway=auto`. |
| `m3000` | Cudy M3000 **v1/v2** (`fleet.yml` 2026-08-31 records `v1`) | `10.254.242.172` | Mesh peer, still a client AP. Learned default via `10.254.12.214`; LAN-source ping 1.1.1.1, DNS + HTTPS verified. |
| `tr3000` | Cudy TR3000 v1 | `10.254.61.115` | Mesh peer. **Only a mesh radio was active before these changes** — no client AP observed. Same downstream internet verification. |

Also observed in `rple`:

- Forwarding was demonstrated independently of the laptop's Wi-Fi:
  `curl -4 --interface enp196s0f4u1 https://example.com` returned 200 while the
  laptop kept Symbio `10.43.1.243` as its preferred default.
- A missing `10.42.12.0/24` `br-lan` connected route was repaired operationally
  after a meshd restart (old deployed startup-flush behaviour). The secondary
  `192.168.1.1` on `m3000-b` was left untouched.
- Recovery snapshot retained on the gateway at `/root/lightning-uplink-rple`
  (`backup/`, `result`, `apply.log`).
- **No AP3000 was observed in this three-node island.** That is an absence of
  observation, not evidence that no AP3000 exists in the household —
  `ap3000-outdoor` is still a fleet entry (§2) and was simply not seen here.

**Not qualified.** `rple` explicitly records *no reboot or power-cycle test*,
installed daemon/helper binaries were intentionally not upgraded, and the
generic `deploy/openwrt/setup-wireless.sh` has no station role, so the manual
one is **not preserved by any supported profile** — see §5.7. That is `z3th`.

## 2. Fleet-file entries — present in files, not observed in the pilot

`deploy/openwrt/fleet-nodes.conf` is the hand-kept **bootstrap/recovery** list,
not proof a node is currently present (nor that a listed node is absent).
`deploy/openwrt/fleet.yml` is a **RF/LAN snapshot observed 2026-08-31** from
the IdentiKey Pirate Radio LAN (`192.168.0.108/24`); overlay `10.254/16` was
unreachable from that vantage, so every `overlay_reachable_from_observer`
there is `false`.

| Entry | Model | Overlay | Status |
|---|---|---|---|
| `ap3000-outdoor` | Cudy AP3000 Outdoor v1 | `10.254.166.226` | Former house gateway (`gateway=auto`, babel `0.0.0.0/0` metric 128) via Origami WAN `10.0.0.239`, **unreachable from the Pirate Radio vantage 2026-08-31**. Client BSSID `80:AF:CA:F2:20:F5`, mesh `82:AF:CA:F2:20:F6`, RF 49% on 2026-08-31. **Not observed in the 2026-09-14 pilot island**; its current whereabouts and power state are unknown. |
| `wr3000s-a` | Cudy WR3000S v1 | `10.254.242.84` | Provisioned 2026-07-01; has no WAN of its own (pkg-cache installs). **BSSID never confirmed** (2026-08-31). Inventory only. |
| `unknown-close` | Cudy (OUI `80:AF:CA`) | — | Client BSSID `80:AF:CA:E7:BA:9C`, Pirate Radio lease `192.168.0.118`, RF 100% on 2026-08-31, all WAN TCP closed (22/80/8080), ping ok. Not bound to a node id. |
| `unknown-d9` | Cudy | — | Client `80:AF:CA:D9:85:AE`, mesh `82:AF:CA:D9:85:AF`, RF 35% on 2026-08-31. Candidate `wr3000s-a` or `tr3000`; **do not add a fleet line until confirmed.** |

Do not relabel the 2026-08-31 snapshot as live, and do not invent node ids for
the two unknown radios.

## 3. SKU / firmware / radio / port facts

**Silicon and bands.** The fleet is OpenWrt on mt76 hardware, **MT7981 /
MT7986** (`deploy/openwrt/README.md`). `setup-wireless.sh` discovers radios by
their UCI `band` option and requires **at least one `2g` and at least one `5g`
radio** — it aborts if either band is missing. It does **not** check an exact
radio count, so a node with a third radio (or a dongle phy) is not rejected;
the script assigns roles to the last matching radio encountered per band. Extra
same-band radios therefore need explicit selection/qualification, not an assumption
that the original assignment stays stable.

Role assignment is driven by `BACKHAUL_BAND`:

- **Default (`BACKHAUL_BAND=5g`)** — 5 GHz 802.11s backhaul (mesh id
  `mjolnir-mesh`, non-DFS ch36) + 2.4 GHz client AP on ch6. This is what the
  fleet runs and what the pilot observed.
- **Supported alternative (`BACKHAUL_BAND=2g`)** — 2.4 GHz backhaul (the
  range/foliage `w1l` choice) + 5 GHz client AP. Supported by the script, not
  in use here.
- Whichever value is chosen is a **fleet-wide constant**: the same
  `BACKHAUL_BAND` *and* backhaul channel on every node, or they do not form
  one island.

Other documented defaults: country `US`; client AP `encryption=none`; factory
default client SSID `⚡`, live fleet override `Lightning Mesh` (open).
5 GHz backhaul throughput was field-validated ~322 Mbit/s on a 4-node bench
2026-07-06 (`setup-wireless.sh`, bead `wai`).

**Firmware/OpenWrt release per node: unknown.** No repo file records it, and
none is inferred here. The only version check the repo defines is the daemon:
`sha256sum /usr/bin/mjolnir-meshd` against `deploy/openwrt/mjolnir-meshd-aarch64`.

**Ports.** Concrete repo evidence is thin:

- `tr3000` — `2.5GbE eth0 + USB3` (`fleet-nodes.conf` notes).
- Each node owns a routed LAN `10.42.<x>.1/24` (`10.42.12.1`, `10.42.242.1`,
  `10.42.166.1` recorded in `fleet.yml`).
- `README.md` documents that a node answers OpenWrt's default `192.168.1.1` on
  its LAN port. That is **documented expected behaviour, not a live reading of
  every node** — `m3000-b`'s `192.168.1.1` **did not answer** from the Pirate
  Radio vantage on 2026-08-31, while the address was still configured as a
  secondary on 2026-09-14. Treat `192.168.1.1` as "usually true, verify per
  unit".
- **Unknown:** per-port WAN/LAN labelling, port counts, link speeds and PoE
  capability for `m3000`, `m3000-b`, `wr3000s-a` and `ap3000-outdoor`. Nothing
  in the repo states them.

**Management paths** (`README.md`, "Reaching & operating nodes"):

1. **Wired/direct** — build machine into the node's LAN port; the node is
   *expected* to answer at `192.168.1.1`. Survives a Wi-Fi/meshd bounce.
   Because the address is shared fleet-wide, the SSH host key changes when you
   swap which node is wired. **Do not run `ssh-keygen -R 192.168.1.1`
   reflexively:** first confirm *which physical unit* is on the other end of
   the cable and obtain/compare its host key out of band (console, a prior
   session over the overlay, or the node's recorded fingerprint). Removing the
   known-hosts entry blind turns a genuine key mismatch into a silent accept.
2. **Mesh jump** — SSH `ProxyJump` through a wired peer to a mesh-only node's
   derived `10.254.x` (SSH is permitted because the mesh iface sits in the
   firewall `lan` zone).
3. **WAN is blocked by default, with documented exceptions.** OpenWrt firewalls
   SSH on the `wan` zone, and the 2026-08-31 snapshot confirms it for
   `unknown-close` (22/80/8080 all closed from the Pirate Radio LAN). But
   `NOTES-dweb-ap3000.md` (2026-08-30) documents `ap3000-outdoor` accepting
   `ssh root@10.0.0.239` on its house WAN lease and being used as the jump host
   to `m3000` — an exception in effect on that unit. Separately,
   `files/usr/sbin/mjolnir-wan-admin` (bead `m0d`) opens a deliberate,
  physically armed WAN SSH window: a WPS press inserts an nft accept of
   TCP/22 from prefixes on the WAN iface for `mjolnir.wan_admin.timeout`
   (default 900 s), dropped by reboot or `fw4` reload. So "the WAN never
   answers" is wrong; "the WAN should not answer unless a node is an exception
   or the window is armed" is right. This existing firewall window is not the
   proposed credential-plus-physical owner recovery ceremony or proof of ownership.

`scp` to these nodes needs `-O` (Dropbear has no SFTP subsystem). Disruptive
changes go through `mjolnir-apply` (snapshot → apply → health gate → rollback),
never an inline SSH mutation.

## 4. Radio-concurrency limits (explicit)

- **Dual-radio budget, as configured.** The supported profiles assign one radio
  to the backhaul and one to the client AP. Under the fleet default the 5 GHz
  radio is the backhaul, so any additional wireless role — client AP, upstream
  station, IoT AP — must fit on the remaining 2.4 GHz radio, or come from extra
  hardware. The user reports dual-radio household nodes; attached extra radios
  have not been inventoried per unit. The script would not reject a third radio.
- **mt76 mesh-point + AP on one radio: field-observed failure, scoped.**
  `setup-wireless.sh` ships the co-located `clientap2g` section **rendered but
  disabled** by default. Bead `mjolnir-mesh-12y` (2026-07-08, closed) records
  the field observation cited as `oaq`: on a **WR3000S**, enabling the
  co-located AP broke the 802.11s **mesh join** itself (`wpad` holds phy0;
  recovery needed AP removal plus a reboot). Bead `ab4`'s contrary claim that
  concurrency works is superseded by that observation. Scope it honestly: this
  is a dated field observation on the mt76 (MT7981/MT7986) firmware in service
  at that time, **not** a proof that no mt76 chip or driver version can ever do
  mesh+AP. It is enough to keep the default off and to refuse to plan on
  concurrency; it is not enough to declare it impossible. Re-validation on a
  recorded firmware version is the way to change this line. Independently,
  "the driver advertises both modes" remains no evidence at all.
- **STA + mesh on the same radio is untested here.** The pilot put the station
  on `radio0` (2.4 GHz) and the mesh on `radio1` (5 GHz) — *different* radios,
  so the `oaq`/`12y` observation does not directly apply, but nothing in the
  repo qualifies a same-radio STA+mesh combination. Do not assume it.
- **USB dongle: one validated device, one route to a third radio.**
  `deploy/openwrt/files/usr/sbin/mjolnir-dongle` supports exactly
  `148f:5370` (Ralink RT5370, 2.4 GHz), role **`ap2g`** — a dedicated 2.4 GHz
  client AP bridged into `br-lan`, validated on TR3000 2026-07-01. There is
  **no validated dongle in an uplink/station role**. Drivers for every table
  entry are preinstalled fleet-wide, so adding a line to that table is the
  whole procedure — but only after validation. A dongle is *one* way to obtain
  a third radio; an onboard third radio, a different SKU, or a wired backhaul
  that frees an onboard radio would serve equally, and none of those is
  excluded by evidence.
- **Extra Ethernet: unknown.** The repo contains no evidence either way about
  USB-Ethernet support or spare uplink ports beyond `tr3000`'s 2.5GbE `eth0` +
  USB3. Absence of a repo entry is not absence of a port — confirm per unit.

## 5. Candidate upstream receiver — constraints

For a node to take the household uplink, the receiver must satisfy:

1. **2.4 GHz reachable, under the current band assignment.** With the fleet
   default (`BACKHAUL_BAND=5g`) the 5 GHz radio is the backhaul, and the
   observed working configuration is a 2.4 GHz station (Symbio, 2026-09-14).
2. **Band-swapped alternative (possible, unqualified).** A 5 GHz upstream is
   not structurally excluded: `BACKHAUL_BAND=2g` moves the mesh to 2.4 GHz and
   frees the 5 GHz radio for a station. This is a supported script mode but has
   **not been qualified for an uplink role here**, costs backhaul throughput
   (§3), and must be applied fleet-wide or the island splits. Treat it as a
   pilot-able alternative, not a recommendation.
3. **Wired backhaul frees radios.** If the backhaul between two nodes can be
   cabled, the radio that carried it becomes available for a station or AP on
   either band. Unqualified here, but it changes the budget in §4 more than any
   dongle does.
4. **Costs that node its client AP on the station's band.** On `m3000-b` the
   pilot disabled `radio0` clientap to free the radio for the station. A
   gateway node therefore serves no 2.4 GHz clients of its own unless a
   validated `ap2g` dongle (or another radio) covers them.
5. **Channel coupling is local to that radio.** A station follows its upstream
   AP's channel, and that radio's other roles share it. It does **not** force
   neighbouring nodes' independent client APs onto that channel — they keep
   their configured channel (fleet 2.4 GHz client AP constant is ch6). Whether
   an off-ch6 upstream causes adjacent/co-channel interference with nearby
   nodes is **unknown and unmeasured**.
6. **Wired upstream alternative.** If an upstream drop can be cabled, a wired
   WAN uplink avoids the RF constraints above. Which physical port to use on
   which SKU is **unknown** (§3) and must be confirmed on the unit.
7. **Re-apply hazard (not a demonstrated failure).** The manual station role is
   **not preserved by any supported profile**: `setup-wireless.sh` has no
   station role, so a re-apply is **not qualified to keep the uplink and may
   remove it**. No re-apply test has been run against the pilot, so this is a
   hazard to design around, not an observed outcome (`rple`, `z3th`).
8. **Reboot behaviour unknown.** No power-cycle test exists for the pilot
   uplink (`rple`).

## 6. Safe temporary Ethernet management topology

This is what `rple` actually did on 2026-09-14, and it is the arrangement to
repeat:

- Laptop **Ethernet** (`enp196s0f4u1` in the pilot) → node LAN port. The laptop
  keeps its **Wi-Fi** association (Symbio `10.43.1.243`) as the **preferred
  default route** throughout. The agent laptop has **one** Wi-Fi radio
  (`AGENTS.md`); joining the mesh client SSID drops its internet and the session
  can no longer report. Never leave that radio parked on a no-internet SSID —
  use `deploy/openwrt/mesh-errand.sh`, which always restores the working
  connection.
- **Preferred addressing: the node's IPv6 link-local on the wired iface.** This
  is the verified path — the pilot reached `m3000-b` at
  `fe80::82af:caff:fee7:ba9d%enp196s0f4u1`. It is per-unit (derived from that
  node's MAC in this observation), but is not a cryptographic identity: addresses
  and MACs can be copied. Verify the SSH host key here too. Using link-local
  addressing installs no competing default route.
- `192.168.1.1` is the fallback, subject to §3: it is shared fleet-wide and is
  not guaranteed to answer. If you use it, identify the physical unit and
  verify the host key out of band before touching `known_hosts` — do not clear
  the entry unconditionally.
- Verify forwarding through the node explicitly bound to the wire
  (`curl -4 --interface <eth-iface> ...`), so a pass cannot be the laptop's own
  Wi-Fi path.
- Ethernet at `192.168.1.1` remains recovery of last resort for a failed
  rollout; in-band staged apply is the normal path.

## 7. Next-hardware requirements

Stated as capability requirements to qualify, not as purchases:

1. **An independent third radio in a station/uplink role** — a way to add
   upstream RF without spending a gateway's client-AP radio. Routes include a
   USB dongle (needs a `mjolnir-dongle` table entry `vid:pid → kmods → role`
   plus a station role handler; today only `ap2g` exists), a SKU with a third
   onboard radio, or freeing a radio via a wired backhaul (§5.3).
2. **Qualified AP+mesh concurrency on a recorded firmware version**, which
   would remove the need for a separate radio entirely. Currently blocked by
   the `12y`/`oaq` field observation (§4) — re-validation, not a purchase, is
   what settles it.
3. **One `ap2g`-class client-AP radio per gateway** that must still serve its
   own clients on the station's band. If the gateway uses its onboard radio for
   the station, that is **one** extra AP dongle, not two. Only `148f:5370`
   qualifies today.
4. **Ethernet backhaul / port maps / PoE** as prospective qualification work:
   confirmed port counts, WAN/LAN labels, link speeds and PoE capability per
   SKU in service, and whether an inter-node or upstream cable run is
   physically available. Currently unknown for everything except `tr3000`.
5. **Node-id binding for the two unknown 2026-08-31 radios** before either is
   counted as household hardware.
6. **Recorded OpenWrt release per node**, so driver-behaviour findings
   (`12y`/`oaq` and successors) can be scoped to a firmware version instead of
   a model name.

---

Qualification of anything above is out of scope here and tracked in
`mjolnir-mesh-z3th` (pilot uplink across reboot and radio re-apply) and
`mjolnir-mesh-lpv` (per-SKU uplink/wired/guest/fallback profiles with failure
recovery). File follow-up work as beads — not as a TODO list in this document.
