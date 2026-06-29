# OpenWrt mt76 node deploy — mjolnir-mesh

For the open WiFi-6 mesh nodes (OpenWrt on mt76 hardware: MT7981 / MT7986,
aarch64). Unlike the MikroTik target there is **no container** — OpenWrt is real
Linux, so the overlay runs as a native static binary supervised by procd,
alongside babeld. See `mjolnir-mesh-0xu` / `mjolnir-mesh-w1l` (Cudy MT7981 fleet)
and `docs/network-coordination/radio-backhaul-and-discovery.md`.

## Build

```sh
deploy/openwrt/build.sh            # -> deploy/openwrt/mjolnir-meshd-aarch64
```

Static `aarch64-unknown-linux-musl` binary (no shared-lib deps), cross-built in
the `messense/rust-musl-cross:aarch64-musl` container (the repo is mounted, so
`target/` is reused and rebuilds are incremental). The artifact is git-ignored —
rebuild as needed. The startup banner stamps the git short-SHA (`MJOLNIR_BUILD`,
`-dirty` if the tree is dirty); see "Verify identity" below.

## Install on a node

One command — idempotent, safe to re-run:

```sh
deploy/openwrt/install-node.sh root@<node-ip>
```

It pushes the binary to `/usr/bin/mjolnir-meshd`, installs the procd init scripts
and UCI config, installs `babeld` (via `apk` on OpenWrt 25.12+, else `opkg`;
`kmod-tun` best-effort — only cross-site iroh tunnels need it), hands babeld
supervision to procd (see below), and enables the meshd service. It does **not**
start meshd — you set peers first.

What lands on the node:

| path | role |
|------|------|
| `/usr/bin/mjolnir-meshd`          | the static daemon |
| `/etc/init.d/mjolnir-meshd`       | procd service (START=95) |
| `/etc/init.d/mjolnir-babeld`      | procd service for babeld (START=96) |
| `/etc/config/mjolnir`             | UCI config (peers, backhaul_iface, mode, …) |
| `/root/setup-wireless.sh`         | 802.11s backhaul + client-AP helper |

### babeld is supervised by procd, not meshd (mjolnir-mesh-m8t)

Split of concerns: **meshd owns the config** — it renders `/etc/mjolnir/babeld.conf`
and triggers `restart` on `mjolnir-babeld` when it changes — and **procd owns the
process** (start on boot, respawn on crash, clean stop). meshd never `fork()`s
babeld itself; that chain orphaned babelds on `SIGKILL`. `install-node.sh`
disables the stock `babeld` service so the two don't both run.

## Configure & run

Edit `/etc/config/mjolnir`, then start the service:

```sh
# this node's id (stable, derived from the persistent secret):
ssh root@<node> 'mjolnir-meshd id --secret-file /etc/mjolnir/secret'

# add the OTHER nodes' ids to /etc/config/mjolnir   (list peer '<64-hex-id>')
# set backhaul_iface: 'br-lan' for the wired-switch bench, or run
#   /root/setup-wireless.sh  then set it to 'br-mesh' for the 802.11s backhaul.

ssh root@<node> 'service mjolnir-meshd start && logread -e mjolnir_meshd'
```

The daemon defaults to `--lan` (offline: mDNS, no relay). meshd self-assigns its
`10.254.0.0/16` backhaul address on `backhaul_iface`, peers discover each other
over the flat L2 via mDNS, babel routes over it as `type wired`, and meshd assigns
the claimed /24's `.1` on `client_iface` as a connected route babel redistributes
(`mjolnir-mesh-e4r`).

Notes:
- Runs as root (needs `CAP_NET_ADMIN` for the backhaul address + TUNs); fine on OpenWrt.
- Persistent `--secret-file` (default `/etc/mjolnir/secret`) → stable node id across reboots.
- For an internet/relay node, set `option mode 'internet'` (and optionally `option relay <url>`).

## Verify identity (mjolnir-mesh-0xu / mjolnir-mesh-auu)

Two routers in one mesh must run the **same binary**. `CARGO_PKG_VERSION` is
`0.1.0` for every build, so the startup banner also logs `MJOLNIR_BUILD` (git
short-SHA). Compare it across nodes before suspecting a transport bug:

```sh
ssh root@<node> 'logread -e "mjolnir-meshd starting"'   # version= build= must match every node
```

A clean SHA (no `-dirty`) means the deployed binary is traceable to a committed
source tree. Pair with a `sha256sum /usr/bin/mjolnir-meshd` check at deploy time.

## Radio side (separate)

The 802.11s mesh + client AP config lives at the OpenWrt/wifi layer — see
`setup-wireless.sh` and the design note. meshd only needs the resulting `br-mesh`
L2 to exist.
