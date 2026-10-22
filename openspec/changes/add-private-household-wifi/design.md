# Private household Wi-Fi

## Pins

The deployed client SSID `Lightning Mesh` is open on the live pair.
This change does not write it. A plan that the owner has not committed
is not a radio change.

Resident credentials follow the uplink precedent. The passphrase is not
one of the four revisioned UCI files (`wireless`, `network`, `firewall`,
`mjolnir`). The plan names a root-only file
`/etc/mjolnir/secrets/wifi/<profile-id>` mode `0600` and a generation
counter in `mjolnir`. Receipts and hello.mesh responses do not contain
the passphrase or that file's bytes.

One resident profile is one house SSID plus one secret generation. A
different SSID is a different profile. Guest is a second profile, off
unless the owner enables it. Guest does not reuse the resident secret.

Isolation is a property of the plan, not a label the UI may add later.
The plan may say `isolated` only when all of these are true:

- Guest clients cannot reach resident addresses on the same node.
- Guest clients cannot reach resident addresses on another node.
- That deny covers IPv4, and IPv6 when the house has IPv6 enabled.
  When IPv6 is not enabled, the plan says so and does not claim an
  IPv6 deny it cannot make.
- The plan does not bridge `br-lan` across nodes and does not put
  resident and guest clients in one L2 segment.
- The only permit from guest to a resident service is an explicit
  named exception in the same plan.

Anything short of that is a refusal. The previous revision stays.
`OpenWrtAdapter` is not called. The builder is pure and tested against
a fake adapter, same as the uplink and wired-profile deciders.

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
