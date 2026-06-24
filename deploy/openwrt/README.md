# OpenWrt mt76 node deploy — mjolnir-mesh

For the open WiFi-6 mesh nodes (OpenWrt on mt76 hardware: MT7981 / MT7986,
aarch64). Unlike the MikroTik target there is **no container** — OpenWrt is real
Linux, so the overlay runs as a native static binary alongside babeld. See
`mjolnir-mesh-dkb` and `docs/network-coordination/radio-backhaul-and-discovery.md`.

## Build

```sh
deploy/openwrt/build.sh            # -> deploy/openwrt/mjolnir-meshd-aarch64
```

Static `aarch64-unknown-linux-musl` binary (no shared-lib deps), built via the
same `messense/rust-musl-cross` cross-image as the MikroTik target. The artifact
is git-ignored — rebuild as needed. Verified to run on arm64 Linux: `--lan` mode,
self-assigns its `10.254.0.0/16` backhaul address, and iroh surfaces it for mDNS.

## Install on a node

```sh
# 1. binary
scp deploy/openwrt/mjolnir-meshd-aarch64 root@<node>:/usr/bin/mjolnir-meshd

# 2. deps on the node
opkg update
opkg install babeld kmod-tun      # babeld = L3 routing; kmod-tun = per-peer /31 TUNs
```

## Run

The daemon defaults to `--lan` (offline: mDNS, no relay). Point `--backhaul-iface`
at the node's mesh L2 — the 802.11s mesh interface, or the bridge that carries it
(e.g. `br-mesh`). meshd self-assigns its `10.254` backhaul address there, peers
discover each other over the flat 802.11s L2 via mDNS, and per-peer tunnels form.

```sh
mjolnir-meshd mesh \
  --secret-file /etc/mjolnir/secret \
  --peer <peer-node-id> [--peer <peer-node-id> ...] \
  --babeld babeld \
  --backhaul-iface br-mesh
```

Notes:
- Runs as root (needs `CAP_NET_ADMIN` for the backhaul address + TUNs); fine on OpenWrt.
- Set a persistent `--secret-file` (or `IROH_SECRET`) so the node id is stable.
- For an internet/relay node instead of LAN, pass `--internet` (or `--relay <url>`).
- A procd init script (`/etc/init.d/`) to supervise it is a TODO for field nodes.

## Radio side (separate)

The 802.11s mesh + client AP config lives at the OpenWrt/wifi layer (see the design
note). meshd only needs the resulting `br-mesh` L2 to exist.
