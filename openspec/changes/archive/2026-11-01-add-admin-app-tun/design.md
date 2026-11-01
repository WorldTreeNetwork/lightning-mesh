# Design

## Address

The app address is `app_ula(guild_id, actor_public_key)`. The actor is the Ed25519 public key of the Admin seed, not the seed itself. Subnet id 1 is the app subnet already pinned by the house-prefix contract. No covering prefix is assigned.

## Packet

The runner is `open_app_tun` in the Admin crate. A test or a second process on the same host calls it. Lightning Admin's guild snapshot does not call it and does not enable the mesh `daemon` feature. The caller passes one house guild id, the actor's 32-byte public key, the peer's 32-byte node id, the peer's app `/128`, and one `DatagramConn` already bound to that node id. The function looks up that exact id. An unknown id returns the computed local address with `up` false, installs no route, and does not use another conn.

On Linux the runner opens one TUN at MTU 1280, assigns only the local app `/128`, and installs only `peer/128` via that device. If the route install fails, the runner removes the address, closes the device, and returns `up` false. `spawn_encap_pair` carries one IP packet to that conn and carries the reply back. The session owns the device, both task handles, and the route it added. `EncapHandles` does not abort on drop, so the session waits on either task. When either task returns, including because the conn closed, the session aborts the sibling, deletes that peer route, and removes the address it assigned. `up` becomes false. The computed address remains. Drop of the session does the same cleanup. A guild or key change is the caller dropping the session; the session does not retarget in place. `up` is true only when `house_addr_usable` is true for the assigned `/128`, the peer `/128` route is installed, and both tasks are still running. `up` is not a claim that the peer answered.

A missing seed, a missing guild id, a TUN open that returns a privilege error, or any non-Linux build returns the computed address only when both inputs exist, with `up` false, and installs no route. None of these paths change Babel or stage the client SSID. The address is not added to an Iroh candidate or bind set.

The second spike run is the evidence for the seam. Each TUN was in its own netns. With the route installed and the pair not running, the ping lost its packet. With the pair running, the echo request and the echo reply were datagrams on the conn labeled with the peer node id. That run did not open a QUIC handshake. This node uses that seam. It does not change the router dial pin.

## Surface

The existing guild surface shows the computed address for the guild it already has selected, when that guild has no outer guild and the actor seed exists. The workplace guild is a house: profile `house`, no outer, `workplace` true. Selecting it shows that house's app `/128`. A nested access guild has an outer and shows no app address. There is no new selector, no new button, and no network-name apply. The surface does not claim the TUN is up.

## Unverified hypotheses

None. The discriminating spike verdict is go: `docs/spikes/admin-tun-encap/finding.md`. The first same-netns ping is not that evidence.
