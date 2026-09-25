## ADDED Requirements

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
