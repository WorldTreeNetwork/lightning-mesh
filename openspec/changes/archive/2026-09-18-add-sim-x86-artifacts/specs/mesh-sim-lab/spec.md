## ADDED Requirements

### Requirement: Daily lab artifacts are x86_64

The sim lab SHALL provide a documented rebuild that produces an
OpenWrt x86_64 guest image and an `x86_64-unknown-linux-musl`
`mjolnir-meshd`. The image SHALL include babeld, `kmod-tun`,
`wpad-mesh-mbedtls`, `kmod-mac80211-hwsim`, dropbear, and the
sim-guest marker `/etc/mjolnir/sim-guest`. Those artifacts SHALL
live under `deploy/sim/` and SHALL NOT replace the shipped aarch64
fleet binary.

#### Scenario: Rebuild

- GIVEN the documented rebuild command
- WHEN it succeeds
- THEN `deploy/sim/` contains an x86_64 OpenWrt image
- AND an `x86_64-unknown-linux-musl` `mjolnir-meshd`
- AND `deploy/openwrt/mjolnir-meshd-aarch64` is still the fleet artifact

#### Scenario: Image is a sim guest

- GIVEN the rebuilt x86_64 image
- WHEN it is inspected or first-booted
- THEN `/etc/mjolnir/sim-guest` is present
- AND `ubus call system board` reports an x86/64 target
