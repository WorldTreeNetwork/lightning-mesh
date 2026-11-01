---
spike_id: admin-tun-encap
hypothesis_id: null
source_synthesis: null
run_at: 2026-11-01T09:48:00Z
time_spent_min: 20
verdict: go
version-pinned: tun 0.8.14, mjolnir-mesh spawn_encap_pair and app_ula, Linux 7.2.11
---

# Spike: a TUN /128 passes one packet to a peer node id

## Claim Tested

A process can open a TUN, assign an app-subnet `/128`, and pass one IP packet through `tun/encap` to the peer selected by a 32-byte node id.

## Minimal Test

`docs/spikes/admin-tun-encap` creates `spikeA` and `spikeB`, assigns `fd81:2b22:8aca:1::1/128` and `fd81:2b22:8aca:1::2/128`, and runs `spawn_encap_pair` in both directions. Each direction's `DatagramConn` is labeled with the other node's id (`11` repeated and `22` repeated). One `ping -6` sends the ICMP echo.

## Evidence

The first run assigned both `/128`s in one netns and treated ping success as proof. That success could have been local delivery. It is not the evidence for seam traversal.

The second run derived both addresses with `app_ula`. Guild `11` repeated and key `ab` repeated produced `fd81:2b22:8aca:1:bd5e:fa7a:5460:697` (`fixture_a_ok=true`). The peer key `cd` repeated produced `fd81:2b22:8aca:1:ef30:9577:4565:e614`. Each TUN was moved into its own netns. Before `spawn_encap_pair`, ping from the first netns lost the only packet (`seam_off_ping_ok=false`). After the pair was attached to the conn labeled `22` repeated, ping received one reply in 0.030 ms. That conn recorded ICMPv6 echo requests (type 128) toward the peer address and the other conn recorded echo replies (type 129) toward the local address (`echo_request_on_peer=2`, `echo_reply_on_peer=2`). A conn labeled `33` repeated was not attached and recorded nothing. Aborting the pair made the next ping fail. The log is `docs/spikes/admin-tun-encap/evidence-r2.log`.

The ULA was not bound as an Iroh candidate. No QUIC handshake was opened.

## Verdict: go

The seam-off ping failed while the host route was installed. The seam-on ping succeeded, and the echo request and reply were the datagrams on the conn labeled with the peer node id. This run did not open an Iroh QUIC handshake. The dial pin stays `10.254`. Binding the ULA for Iroh is `remove-v4-backhaul-pin`, which this spike does not unlock.
