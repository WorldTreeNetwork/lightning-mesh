# client-roam

## ADDED Requirements

### Requirement: Any node confirms a gossiped lease

A node SHALL confirm, over DHCP, the IPv4 address the mesh lease book holds
for a client MAC, when that address lies in `10.42.0.0/16` outside the node's
own client subnet. The reply SHALL carry netmask `255.255.255.0` and the
router and DNS server `10.42.<x>.1` of the address's own `/24`. A node SHALL
allocate new addresses only from its own client subnet. No node SHALL be
elected to serve DHCP for another.

#### Scenario: Re-DHCP after a hop keeps the address

- GIVEN node-a leased `10.42.61.106` to a phone and node-b has that lease
  in its lease book
- WHEN the phone sends DHCPREQUEST for `10.42.61.106` on node-b
- THEN node-b answers DHCPACK for `10.42.61.106`
- AND the router option is `10.42.61.1`

#### Scenario: DISCOVER after a hop offers the same address

- GIVEN the same lease on node-b
- WHEN the phone sends DHCPDISCOVER on node-b
- THEN node-b offers `10.42.61.106`, not an address in its own subnet

#### Scenario: A new client is allocated locally

- GIVEN a MAC with no entry in the lease book
- WHEN it sends DHCPDISCOVER on node-b
- THEN node-b offers an address from node-b's own client subnet

#### Scenario: A reservation arrives without a dnsmasq restart

- GIVEN node-b is serving DHCP
- WHEN a lease for a new MAC arrives by gossip
- THEN node-b confirms that address on the next request
- AND dnsmasq was signalled, not restarted

### Requirement: Leases reach the lease book from the lease file

A node SHALL publish a lease to the mesh lease book when dnsmasq's lease file
gains an entry in `10.42.0.0/16`, or when an existing entry's expiry
advances. It SHALL NOT depend on a dnsmasq hook script. On its first read
after start it SHALL NOT re-stamp an entry the book already holds with the
same address.

#### Scenario: A fresh lease is gossiped

- GIVEN dnsmasq on node-a writes a lease for a new phone
- WHEN one ingest interval has passed
- THEN node-a's lease book holds that MAC and address
- AND node-b's hosts file lists that MAC and address after gossip delivery

#### Scenario: Restart does not reorder leases

- GIVEN node-a's lease file and lease book both hold MAC `m` at address `p`
- AND the book's entry was last written by node-b
- WHEN meshd on node-a restarts
- THEN the book entry for `m` is unchanged

#### Scenario: The jail does not stop ingest

- GIVEN dnsmasq runs under OpenWrt's `ujail`
- WHEN a client takes a lease
- THEN the lease is ingested

### Requirement: Client APs advertise their mesh neighbours

A node SHALL enable 802.11k neighbour reports and 802.11v BSS transition
management on each client AP, and SHALL install as neighbours the client APs
of other nodes that carry the same SSID. It SHALL do so without reloading
the radio. It SHALL NOT disassociate or steer a client. A neighbour whose
owner has stopped refreshing it SHALL be removed.

#### Scenario: Two nodes list each other

- GIVEN node-a and node-b each run a client AP with SSID `Lightning Mesh`
- WHEN both have gossiped their neighbour reports
- THEN `rrm_nr_list` on node-a contains node-b's BSSID
- AND `rrm_nr_list` on node-b contains node-a's BSSID

#### Scenario: A different SSID is not a neighbour

- GIVEN node-c runs a client AP with another SSID
- WHEN its report arrives
- THEN node-a does not install it

#### Scenario: hostapd restart is repaired

- GIVEN neighbours were installed
- WHEN hostapd restarts and forgets them
- THEN the node installs them again within one reconcile interval

#### Scenario: A dead node leaves the list

- GIVEN node-b stops refreshing its report
- WHEN the report's lifetime has passed
- THEN node-a removes node-b's BSSID from its neighbour list

### Requirement: Proxy ARP for a roamed client is not delayed

A node SHALL set `proxy_delay` to `0` on the client bridge.

#### Scenario: Gateway ARP after a hop

- GIVEN a roamed phone ARPs for its home gateway
- WHEN the request reaches the node it is on
- THEN the node replies without the kernel's proxy delay

### Requirement: Every client bridge answers on one gateway MAC

A node SHALL set its client bridge's MAC to the fleet gateway MAC
`02:4c:6d:65:73:68`, at runtime, without reloading the network. It SHALL
re-assert it if something resets it. A node that detects another node's
gateway on its own client segment SHALL put its hardware MAC back.

#### Scenario: Gateway check passes after a hop

- GIVEN a phone leased on node-a has the gateway MAC cached
- WHEN it associates to node-b and sends a unicast ARP for its gateway to
  that MAC
- THEN node-b replies from the same MAC

#### Scenario: Existing clients keep their gateway

- GIVEN a client that cached the bridge's hardware MAC as its gateway
- WHEN the node sets the gateway MAC
- THEN that client's traffic to the gateway still flows

#### Scenario: Merged copper

- GIVEN two nodes' LAN ports are cabled to one switch
- WHEN a node sees the other's gateway address on its client bridge
- THEN it restores its hardware MAC
