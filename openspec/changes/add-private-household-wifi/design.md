# Private household Wi-Fi

## Pins

The deployed client SSID `Lightning Mesh` is open on the live pair.
This change does not write it. A plan that the owner has not committed
is not a radio change.

Resident credentials follow the uplink precedent, with the hostapd
file shape pinned. v1 encryption is `psk2`. The `wifi-iface` carries
no `option key`, and there is no `wifi-station` key. It names a
root-only file through `option wpa_psk_file`. That file is
`/etc/mjolnir/secrets/wifi/<profile-id>`, mode `0600`. `<profile-id>`
is derived from the SSID and the role (`resident` or `guest`), never
from the secret. Each line is `00:00:00:00:00:00 <passphrase>` so
hostapd accepts it for every station. SAE and `sae_password_file` are
not v1. A 6 GHz iface, which skips the PSK file, is refused.

The transaction snapshots and restores that file with the UCI
revision. A passphrase-only change increments `mjolnir` `secret_generation`
so the revision moves. No revision, manifest, or receipt carries an
unkeyed hash of the file. hello.mesh responses do not contain the
passphrase or the file bytes.

The builder returns a plan type that `OpenWrtAdapter` cannot submit,
the same kind of gate as uplink `ProvesUpstream`.

One resident profile is one house SSID plus one secret generation. A
different SSID is a different profile. The cutover name, including
whether it stays `Lightning Mesh`, is an input. This change does not
pick it (`mjolnir-mesh-lpv`). Guest is a second profile, off unless
the owner enables it. Guest does not reuse the resident secret.

Isolation is a property of the plan, not a label the UI may add later.
The plan may say `isolated` only when all of these are true:

- Each guest-serving node denies guest destinations that fall in the
  house aggregates. For IPv4 those are `10.42.0.0/16` and
  `10.254.0.0/16`. For IPv6 they are the house ULA and the house GUA
  prefixes when IPv6 is enabled. The deny is not a list of current
  peers and not a ban on forwarding to the mesh. Traffic to the
  internet (anything outside those aggregates, including a double-NAT
  upstream) stays allowed, so a guest on a satellite still reaches
  the gateway's WAN.
- The same-node deny includes the node's own addresses. Guest input
  accepts only DHCP, DNS, and, when IPv6 is enabled, ND and RA.
- The default guest plan names no hello.mesh exception. Guests do not
  reach `10.42.x.1:80` until a later change. Anonymous hub browsing
  stays on the resident network, or on the open SSID before commit.
- IPv6-enabled is read from the committed `network` and `mjolnir`
  config. If the builder cannot tell, it refuses. A later plan that
  enables IPv6 while a committed guest profile has no IPv6 deny is
  refused unless that same revision carries the IPv6 deny.
- Guest addresses are a per-node slice inside `10.42.0.0/16` that is
  not the resident `/24`. They are not bridged onto `br-lan`. `roam`
  does not install a guest `/32` on `br-lan` and does not export one.
  Guests do not roam in this change. They re-DHCP on the node they
  join.
- The only extra permit is an explicit named service exception in the
  same plan.

Anything short of that is a refusal. The previous revision stays.
The builder is pure and tested against a fake adapter.

Creating an identity is not an input to either profile. Internet
permission follows the network plan, not a membership record.

The open-SSID bootstrap is explicit. The default plan object records
`resident-private` as the house intention and `radio: unchanged` until
`commit: true`. A committed plan is the one that would replace the open
SSID. This change implements the decision and the refusal rules. It
does not perform the replacement.

## Not owed

- Choosing bands, channels, or schedules.
- Device-only SSIDs, SQM, VPN, or multicast reflection.
- Rendering nftables on a router. The plan states the denies. A later
  apply slice renders them when live probes exist.
