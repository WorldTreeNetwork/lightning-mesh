# Tasks

Authoritative owed work is `mjolnir-mesh-sim.2`.

- [x] Human activation (banner to ACTIVE BUILD) — Duke said `activate all` 2026-09-18
- [x] `deploy/sim/build-meshd.sh` produces `deploy/sim/mjolnir-meshd-x86_64` (`x86_64-unknown-linux-musl` static-pie). hello via `deploy/sim/build-hello.sh`
- [x] OpenWrt x86_64 image (`deploy/sim/build-image.sh`) with babeld, kmod-tun, wpad-mesh-mbedtls, kmod-mac80211-hwsim, dropbear, `/etc/mjolnir/sim-guest`
- [x] `deploy/sim/README.md` names the rebuild commands and artifact paths
- [x] Metal `deploy/openwrt/mjolnir-meshd-aarch64` remains ARM; fleet scripts do not consume `deploy/sim/*-x86_64`

Not owed here (bullets, not boxes):

- libvirt XML (`sim.3`)
- vwifi
- Fold (after act lands)
