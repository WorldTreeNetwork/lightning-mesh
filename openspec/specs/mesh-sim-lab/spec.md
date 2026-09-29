# mesh-sim-lab

Laptop-local OpenWrt fleet on libvirt/QEMU. Guests are generic virt
machines (daily PC/q35 x86_64), not Filogic/Cudy sysupgrade. Air is
vwifi + `mac80211_hwsim`; ethernet NAT is virtio; SSH and vwifi TCP
ride a third net. Sim wireless/apply runs only on a positive sim-guest
marker. A green sim roam does not close metal roam beads. Folded from
`add-mesh-sim-lab` (2026-09-17), `add-sim-vwifi-air` (2026-09-18),
`add-sim-node-profile` (2026-09-18), `add-sim-nat-topology` (2026-09-24),
`add-sim-roam-keep-ip` (2026-09-24), `add-sim-e2e-keep-ip` (2026-09-25),
`add-sim-e2e-runner` (2026-09-25), `add-sim-e2e-nat` (2026-09-25),
`add-sim-e2e-dhcp-crdt` (2026-09-25), `add-sim-e2e-fade` (2026-09-25),
`add-sim-e2e-dna` (2026-09-25), `add-sim-e2e-apply-rollback` (2026-09-29).

Contract: `deploy/sim/README.md`. Layout is `deploy/sim/`, not under
`deploy/openwrt/`. Operational shape copies `morphist-win11` /
`admin/scripts/windows-build/`.

## Requirements

### Requirement: Lab guests are generic OpenWrt, not Filogic

The mesh sim lab SHALL run OpenWrt as QEMU/libvirt guests on a generic
virtual machine. Daily x86_64 KVM guests SHALL use a PC/q35 machine.
QEMU `-M virt` SHALL be used only for an optional aarch64 `armsr`
smoke guest. The lab SHALL NOT boot MediaTek Filogic / Cudy sysupgrade
images or require MT7981 machine models.

#### Scenario: Daily guest matches the host

- GIVEN this workstation is x86_64 with KVM
- WHEN the daily sim lab starts
- THEN guests are OpenWrt x86_64 on a PC/q35 machine
- AND they run an `x86_64-unknown-linux-musl` `mjolnir-meshd`
- AND they are not the Cudy sysupgrade artifact in `deploy/openwrt/vendor-firmware/`
- AND the machine type is not QEMU `virt`

#### Scenario: Filogic image is refused

- GIVEN a Cudy `*-squashfs-sysupgrade.bin`
- WHEN an operator asks the lab to boot it in QEMU
- THEN the lab documentation and scripts refuse
- AND they point at the generic OpenWrt guest instead

### Requirement: Air is vwifi, ethernet is virtio, mgmt is separate

Simulated 802.11 SHALL use `mac80211_hwsim` radios whose frames are
relayed by vwifi. WAN/LAN/NAT SHALL use virtio ethernet. SSH and
vwifi control SHALL use a management network that is not the
simulated wifi.

#### Scenario: Two radios

- GIVEN a mesh node guest
- WHEN vwifi has attached radios
- THEN one radio can run the client AP
- AND another can run 802.11s backhaul
- AND `setup-wireless.sh` can discover a `band=2g` and a `band=5g` device

#### Scenario: Link strength is coordinates

- GIVEN two node guests with 802.11s up
- WHEN an operator moves a guest with `vwifi-ctrl`
- THEN packet loss or reported signal between those radios changes
- AND the control session used the management network, not a wifi STA

#### Scenario: Double NAT still meshes on the air

- GIVEN node-a WAN on an ISP-CPE LAN
- AND node-b WAN behind a second NAT
- WHEN 802.11s is up
- THEN overlay `10.254` reachability does not require a WAN hole-punch from B to A

#### Scenario: Management NIC is not the mesh dataplane

- GIVEN two node guests with mgmt SSH working
- AND 802.11s/vwifi currently providing babel adjacency and `10.254` ping
- WHEN vwifi/backhaul reachability is removed or partitioned
- THEN mgmt SSH still works
- AND babel adjacency between those nodes is gone
- AND peer `10.254` ping fails
- AND a harness that still sees overlay only via the mgmt NIC is a failure

### Requirement: Sim profile cannot apply on metal

A wireless or apply profile written for the sim lab SHALL run only on
a guest that presents a positive sim-guest identity planted by the
lab image. The profile SHALL complete that check before any UCI or
`wireless` write. Absence of the marker SHALL be refusal on every
physical OpenWrt board, including Cudy Filogic, OpenWrt One, Banana
Pi, and boards not yet in the fleet. Fleet scripts under
`deploy/openwrt/` SHALL NOT treat sim guests as `fleet-nodes.conf`
entries.

#### Scenario: Marker required before any wireless write

- GIVEN a node that does not have the sim-guest marker
- WHEN an operator runs the sim wireless/apply profile on it
- THEN the profile aborts before any UCI or `wireless` write
- AND live `MESH_ID` / client SSID are untouched

#### Scenario: Non-Cudy metal is also refused

- GIVEN a physical OpenWrt board that is not Cudy Filogic (for example
  OpenWrt One or Banana Pi)
- AND it lacks the sim-guest marker
- WHEN the sim profile is run
- THEN it is refused the same way as a Cudy board

#### Scenario: Sim guest is identified positively

- GIVEN a lab guest whose image planted the sim-guest marker
- AND `ubus call system board` reports an x86/64 or armsr target
- WHEN the sim profile is run
- THEN the profile may change `wireless` on that guest

#### Scenario: Fleet rollout does not see the lab

- GIVEN `deploy/sim/` exists
- WHEN `update-fleet.sh` walks `fleet-nodes.conf`
- THEN it does not start, stop, or apply to a sim domain

### Requirement: Sim evidence is not metal roam proof

A passing sim roam or NAT scenario SHALL NOT close beads `mjolnir-mesh-5wc`,
`mjolnir-mesh-wvg`, `mjolnir-mesh-wvg.1`, or `mjolnir-mesh-sz9.1`.
Harness output MAY name those beads as the scenario imitated.

#### Scenario: Keep-IP harness passes

- GIVEN a Linux STA associated to node-a with a vended `10.42` address
- WHEN it reassociates to node-b
- THEN the harness records whether the IPv4 stayed, whether proto 158
  `/32` moved, whether LeaseBook kept the MAC→IP, and the egress gap
- AND `5wc` remains open until a physical client walk

#### Scenario: Linux STA is not an iPhone

- GIVEN the sim phone guest uses `wpa_supplicant`
- WHEN gateway MAC via proxy-ARP changes
- THEN the harness does not treat that as `sz9.1` closed
- AND iPhone DNAv4 remains a metal / `sz9.1` concern

### Requirement: Lab operations copy the Windows VM shape

Starting and stopping the lab SHALL use named libvirt domains and
SSH BatchMode, like `morphist-win11` / `admin/scripts/windows-build/`.
The lab SHALL live under `deploy/sim/`.

#### Scenario: Cold start

- GIVEN domains are shut off
- WHEN an operator runs the documented start command
- THEN mgmt DHCP/leases appear
- AND `ssh -o BatchMode=yes` to a node guest succeeds without a TTY prompt

#### Scenario: Chat uplink survives

- GIVEN the operator workstation uses a working Wi-Fi for the agent session
- WHEN the sim lab is running
- THEN the lab does not require that workstation to join the client SSID
- AND `mesh-errand.sh` is not the path to reach sim guests

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

### Requirement: Named libvirt domains boot on mgmt

The sim lab SHALL define named libvirt networks and q35 domains so an
operator can start guests and SSH on the management net (`10.99.0.0/24`)
with BatchMode, without joining the client SSID. The management prefix
SHALL NOT overlap Docker or other host bridges.

#### Scenario: Cold start two nodes

- GIVEN the sim x86 image exists
- WHEN the documented start command runs
- THEN node-a and node-b obtain mgmt leases
- AND `ssh -o BatchMode=yes root@<mgmt>` succeeds

### Requirement: vwifi is the lab air

Node guests SHALL run `vwifi-client` against a host `vwifi-server` with
two `mac80211_hwsim` radios. 802.11s SHALL establish between node-a and
node-b. `vwifi-ctrl` SHALL be able to change link loss. vwifi TCP SHALL
use the management network.

#### Scenario: Mesh plink

- GIVEN two node guests with vwifi up
- WHEN both mesh points share mesh_id and channel
- THEN `iw` reports ESTAB between them

### Requirement: install-node runs on sim guests

`install-node.sh` SHALL be able to stage and apply meshd/babeld on a
sim guest over mgmt SSH. The sim wireless profile SHALL require the
sim-guest marker before any UCI/`wireless` write.

#### Scenario: Apply on a marked guest

- GIVEN a sim guest with `/etc/mjolnir/sim-guest`
- WHEN install-node runs against its mgmt address
- THEN meshd and babeld are present
- AND wireless devices include band=2g and band=5g hwsim radios

### Requirement: Double-NAT nodes still mesh on the air

node-a SHALL be able to take WAN on the ISP-CPE LAN with `gateway=auto`.
node-b SHALL be able to take WAN behind a second NAT without exporting
default. Overlay `10.254` reachability SHALL NOT require a WAN
hole-punch from B to A.

#### Scenario: Overlay without WAN path

- GIVEN 802.11s ESTAB a↔b
- AND B's WAN is double-NAT
- WHEN an operator pings A's `10.254` from B
- THEN it succeeds
- AND the path is not B-WAN → A-WAN

### Requirement: Keep-IP harness is a measured STA hop

The sim lab SHALL provide a scripted Linux STA (same sim image, STA-only)
that associates to node-a, hops to node-b, and records an ordered
pre-hop baseline plus post-hop observations. Passing SHALL NOT close
beads `mjolnir-mesh-5wc`, `mjolnir-mesh-wvg`, `mjolnir-mesh-wvg.1`, or
`mjolnir-mesh-sz9.1`. A roam command with no observed association
transition SHALL NOT be a pass.

#### Scenario: Scripted hop green

- GIVEN a STA associated to node-a with a vended `10.42` address
- AND a pre-hop baseline of MAC, IPv4, home node, LeaseBook LWW entry,
  `iw station dump` on A (present) and B (absent), usable `ip neigh` on A,
  proto 158 `/32` absent on both, and a running sequenced egress probe
- WHEN the harness hops it to node-b (log names `wpa_cli roam` and/or
  `vwifi-ctrl`)
- THEN IPv4 is unchanged
- AND the STA is in B's `iw station dump` and not in A's
- AND proto 158 `/32` is present on B only and absent on A
- AND LeaseBook still maps that MAC to the same IPv4 (current LWW)
- AND the probe logs last-success-before, first-success-after, loss
  count, and observed egress node B
- AND those metal roam beads remain open

#### Scenario: Command without transition is red

- GIVEN the harness issues a roam command
- WHEN B's `iw station dump` does not contain the STA or A still does
- THEN the harness exits non-zero

#### Scenario: Stale /32 on A is red

- GIVEN the hop
- WHEN A still has proto 158 `/32` for the STA IP
- THEN the harness exits non-zero even if B also has it

#### Scenario: Lease divergence is red

- GIVEN the hop
- WHEN LeaseBook maps the STA MAC to a different IPv4 than baseline
- THEN the harness exits non-zero

#### Scenario: Egress does not resume through B is red

- GIVEN a sequenced probe across the hop
- WHEN there is no first-success-after via B
- THEN the harness exits non-zero

### Requirement: Keep-IP e2e fails while A still announces proto 158

The sim e2e runner SHALL expose `keep-ip` that hops a Linux STA A→B.
The run SHALL fail if proto 158 `/32` for the STA IP remains on A
after the hop, even if B also has it. The run SHALL NOT close
`mjolnir-mesh-5wc`, `wvg`, `wvg.1`, or `sz9.1`. Dual-158 SHALL NOT
be marked expected-fail.

#### Scenario: Dual /32 is red

- GIVEN a STA that kept its IPv4 across A→B
- AND proto 158 `/32` is present on both A and B
- WHEN `deploy/sim/e2e/run.sh keep-ip` finishes
- THEN the command exits non-zero
- AND `5wc` remains open

### Requirement: Health e2e is a gated command

The sim lab SHALL provide `deploy/sim/e2e/run.sh health` that checks
mgmt SSH, the sim-guest marker, 802.11s ESTAB, and overlay `10.254`
ping. Partitioning vwifi SHALL fail overlay ping while mgmt SSH still
works. The runner SHALL NOT close beads.

#### Scenario: Health pass

- GIVEN the q35 guests are running and vwifi is up
- WHEN an operator runs `deploy/sim/e2e/run.sh health`
- THEN SSH to 10.99.0.10–14 succeeds in BatchMode
- AND each node guest has `/etc/mjolnir/sim-guest`
- AND 802.11s is ESTAB between node-a and node-b
- AND `10.254` ping a↔b succeeds

#### Scenario: Partition fails overlay not SSH

- GIVEN health preconditions
- WHEN vwifi-server is stopped
- THEN overlay `10.254` ping fails
- AND mgmt SSH still succeeds
- AND the runner restores vwifi before exit

### Requirement: NAT e2e is a gated command

`deploy/sim/e2e/run.sh nat` SHALL check that node-a WAN is on the
ISP LAN (`192.168.1.0/24`) with `gateway=auto`, node-b WAN is on
`192.168.50.0/24` without default export, overlay `10.254` traceroute
is one hop, and node-a cannot ping node-b's WAN address.

#### Scenario: Overlay not WAN

- GIVEN the household NAT fixture
- WHEN `deploy/sim/e2e/run.sh nat` runs
- THEN overlay traceroute a↔b is 1 hop
- AND ping from A to B's WAN address fails

### Requirement: DHCP CRDT e2e is a gated command

`deploy/sim/e2e/run.sh dhcp-crdt` SHALL hop or place a STA on node-b
after node-a vended its IPv4, then DHCPREQUEST that address. The run
SHALL fail if node-b offers a different IPv4 or if `mjolnir-roam.conf`
on B does not contain the STA MAC→IP. The run SHALL NOT close
`mjolnir-mesh-wvg.4`.

#### Scenario: B ACKs A's lease

- GIVEN a STA whose IPv4 was vended on node-a
- WHEN it DHCPREQUESTs on node-b
- THEN the IPv4 is unchanged
- AND B's roam dhcp-host file lists that MAC and IP

### Requirement: Fade e2e is a gated command

`deploy/sim/e2e/run.sh fade` SHALL increase vwifi distance until overlay
`10.254` ping fails, then restore coordinates and require reconvergence.
The run SHALL fail if overlay stays up. Killing vwifi-server is the
health partition, not this suite.

#### Scenario: Distance fade drops overlay

- GIVEN overlay ping a↔b works
- WHEN vwifi-ctrl moves guests far apart
- THEN overlay ping fails while mgmt SSH lives
- AND restoring coordinates restores overlay ping

### Requirement: DNA e2e is a gated command

`deploy/sim/e2e/run.sh dna` SHALL run the RFC 4436 INIT-REBOOT probe
and SHALL NOT close `mjolnir-mesh-sz9.1`.

#### Scenario: Observation recorded

- GIVEN a STA with a lease
- WHEN `run.sh dna` finishes successfully
- THEN keep vs hole is logged
- AND `sz9.1` remains open

### Requirement: Apply-rollback e2e is a gated command

`deploy/sim/e2e/run.sh apply-rollback` SHALL run `mjolnir-apply` on a
sim guest that presents `/etc/mjolnir/sim-guest`. The run SHALL
snapshot current allowlisted UCI, stage a UCI mutation that fails
closed before commit, and require the snapshot to be restored. The
command SHALL fail if the result is not `ROLLED_BACK`, if UCI still
contains the staged change, or if overlay `10.254` ping a↔b does not
succeed after restore. The runner SHALL NOT close beads. Metal apply
remains a separate hardware gate. The harness SHALL NOT bounce
sim radios (`RUN_WIRELESS` / `wifi reload`) — dedicated hostapd is
the client AP.

#### Scenario: Bad UCI rolls back

- GIVEN node-a is a marked sim guest with overlay ping to node-b
- WHEN `deploy/sim/e2e/run.sh apply-rollback` runs
- THEN `mjolnir-apply` writes `ROLLED_BACK`
- AND the staged UCI mutation is absent
- AND overlay `10.254` ping a↔b succeeds
- AND `lpv` / `z3th` remain open

#### Scenario: Marker still required

- GIVEN a node without `/etc/mjolnir/sim-guest`
- WHEN the sim apply-rollback profile would write UCI
- THEN it aborts before mutation
