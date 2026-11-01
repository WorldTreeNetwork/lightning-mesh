## ADDED Requirements

### Requirement: An app opens a house /128 through mjolnir-app-tun

`mjolnir-app-tun` SHALL compute `app_ula` from a 32-byte house guild id and a 32-byte app public key, using subnet id 1. It SHALL open one Linux TUN at MTU 1280, assign only that `/128`, install only one peer `/128` via that device, and pass one IP packet through `spawn_encap_pair` on the single `DatagramConn` the caller supplies. An unknown peer node id SHALL NOT select another conn. The session SHALL own the device, both encap tasks, and the routes it added. Drop, or either encap task returning, SHALL remove that address and those routes. The crate SHALL NOT link Iroh, read a guild log, assign a covering prefix, or add the address to an Iroh bind set. A non-Linux build SHALL report the computed address with up false and SHALL install nothing.

#### Scenario: Fixture app address

- GIVEN guild id `11` repeated to 32 bytes and app key `ab` repeated to 32 bytes
- WHEN the crate computes the app address
- THEN the address is `fd81:2b22:8aca:1:bd5e:fa7a:5460:697`

#### Scenario: One packet uses the supplied conn

- GIVEN one open session and one `DatagramConn`
- WHEN one IP packet is written toward the peer `/128`
- THEN that conn observes the packet and the crate has not opened an Iroh connection

#### Scenario: Prefix bytes install a foreign /48

- GIVEN the session was opened for guild `11` repeated and is up
- WHEN the caller passes the 6-byte prefix of guild `22` repeated
- THEN `fd8f:78a9:2caa::/48` is a device route on that TUN and the session's own `/48` is not assigned
