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
is the lowercase hex of a domain-separated blake3 over the role
(`resident` or `guest`) and the SSID bytes, the same length-prefix
shape as uplink `Candidate::id`, domain `mjolnir-wifi-profile`. It is
never the secret and never the raw SSID, so `/`, `..`, and non-UTF-8
bytes cannot leave `/etc/mjolnir/secrets/wifi/`. The file is exactly
one line, `00:00:00:00:00:00 <passphrase>`, so
hostapd accepts it for every station. The passphrase is uplink
`Passphrase` (`crates/mjolnir-apply/src/uplink.rs`): printable ASCII
`0x20..=0x7e` of length 8..=63, or 64 hex digits, so a newline cannot
become a second file line. SAE and `sae_password_file` are not v1. A
6 GHz iface, which skips the PSK file, is refused.

The transaction snapshots and restores that file with the UCI
revision. A passphrase-only change increments `mjolnir` `secret_generation`
so the revision moves. No revision, manifest, or receipt carries an
unkeyed hash of the file. hello.mesh responses do not contain the
passphrase or the file bytes.

The builder returns a plan type that `OpenWrtAdapter` cannot submit,
the same kind of gate as uplink `ProvesUpstream`. The gate's adapter
contract is: a revision that changes a `wifi-iface`'s `wpa_psk_file`
or its `secret_generation` makes hostapd re-read that file on apply
and on restore. The plan names those ifaces.

One resident profile is one house SSID plus one secret generation. A
different SSID is a different profile. The cutover name, including
whether it stays `Lightning Mesh`, is an input. This change does not
pick it (`mjolnir-mesh-lpv`). Guest is a second profile, off unless
the owner enables it. Guest does not reuse the resident secret.

Isolation is a property of the plan, not a label the UI may add later.
The plan may say `isolated` only when all of these are true:

- Each guest-serving node denies forwarded guest destinations in
  the committed mesh client space and in `10.254.0.0/16`. The
  default client space is `10.42.0.0/16`
  (`crates/mjolnir-mesh/src/alloc.rs` `DEFAULT_MESH_SPACE`). The
  deny is not a list of current peers and not a ban on forwarding
  to the mesh. A guest CIDR outside that client space is refused.
- Every house node rejects INPUT whose source is in the committed
  guest-source class, except DHCP and DNS on that node's own guest
  interface. That covers the gateway's WAN address, `wwan`, recovery
  `192.168.1.1` when it is an address of the node, and any public
  address configured on a house node. A satellite guest forwarded
  toward the internet is FORWARD, not INPUT, and stays allowed.
- Each node's copy of the guest-source class lives in its committed
  `mjolnir` config. The builder enables a guest `wifi-iface` on node
  N only when every house node's committed config, passed in as
  input, already rejects INPUT from N's guest CIDR. A missing,
  unreachable, or stale node refuses the plan. The fleet reject is
  committed before any guest SSID. A newly joined node is not counted
  until its committed config carries the class. Any later revision
  on a house node that drops a guest CIDR from that node's class,
  while any house node still has a guest SSID committed for that
  CIDR, is refused. The previous revision stays. This is any
  builder's rewrite of `mjolnir`, not only a restore. In-crate
  writers that edit named sections keep the class. Hand `uci`,
  `meshctl`, and deploy scripts are not builders. The
  guest-enable submitter re-reads every peer revision it built
  against immediately before it submits the guest SSID, and refuses
  if any moved. The residual window is two transactions that both
  pass that check before either commits.
- A node that joins after a guest SSID is committed is not covered
  until its committed config carries the class, and a new guest plan
  is refused while it lacks the class. The export hold, including
  the peer-file install path, is `mjolnir-mesh-fyby.4`. This change
  does not implement admission.
- The upstream LAN beyond a house node's own addresses is internet
  for this change, not resident space. Devices that live only on the
  ISP router's LAN are outside the mesh client aggregates.
- The same-node deny includes the node's own addresses. Guest input
  accepts only DHCP and DNS. The guest interface's device sets
  `ipv6` to `0`, so the guest link has no IPv6 address, including
  `fe80::`. IPv6 neighbor discovery is out of this change because a
  guest profile on an IPv6-enabled house is refused, and the device
  flag covers the link the kernel would address anyway.
- The default guest plan names no hello.mesh exception. Guests do not
  reach `10.42.x.1:80` until a later change. Anonymous hub browsing
  stays on the resident network, or on the open SSID before commit.
- IPv6 is enabled when the committed config shows any of
  `network.globals` `ula_prefix`, `ip6assign` on any interface, a
  `wan6` interface or `proto` `dhcpv6`, or odhcpd `ra` or `dhcpv6`
  server on a client interface. If those configs are not passed in,
  the builder refuses. This change refuses a guest profile when that
  predicate is true. On today's fleet that predicate is true
  (`ula_prefix`, `wan6`), so a guest plan is refused until IPv6 is
  stripped or a later change adds a v6 deny. A guest `wifi-iface`
  carries no `ip6assign`.
  It does not invent a house GUA or a shared ULA list. A later change
  can add the v6 deny. Enabling IPv6 while a guest profile is already
  committed is refused unless that same revision drops the guest
  profile.
- The guest slice is a CIDR the CRDT already claimed under
  `/subnets/{cidr}`. The builder receives it as an input and does not
  allocate. The guest-source class is those CIDRs as listed in each
  node's committed `mjolnir` config. There is no separate fleet plan
  object. A slice that is not in that class is not yet guest traffic.
- Babel redistribution of that CIDR, so the gateway has a return
  route, is a meshd follow-up. This builder does not claim the route
  exists. The satellite-internet scenario is a forward-policy allow.
- Guest addresses are not the resident `/24`. They are not bridged
  onto `br-lan`. `guest_routes` (`crates/mjolnir-mesh/src/roam.rs`)
  excludes addresses in the committed guest-source class, so a stale
  `10.42` lease in that class does not get a proto-158 `/32`. Guests
  do not roam in this change. They re-DHCP on the node they join.
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
- Rendering nftables on a router. The plan states the denies.
  `mjolnir-mesh-nx1k` renders them when live probes exist. A plan
  that says `isolated` is not yet enforced on the router.
- Removing an old secret file after an SSID change. The old file
  stays so restore can put it back.
- Same-SSID client isolation (`option isolate`). Guests on one SSID
  can reach each other in this change.
- Keeping `/etc/mjolnir/secrets/` across sysupgrade. That is `lpv`
  or the deploy path.
