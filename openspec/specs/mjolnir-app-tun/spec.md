# mjolnir-app-tun

`mjolnir-app-tun` computes a house app `/128` and opens one Linux TUN
session over the single `DatagramConn` the caller supplies. `mjolnir-mesh`
and Lightning Admin depend on it. The crate does not depend on
`mjolnir-mesh`, does not link Iroh, and does not enable feature
`daemon`. Cancellation is `EncapHandles::abort`. Folded from
`extract-mjolnir-app-tun` (2026-11-01).

Code: `crates/mjolnir-app-tun` (`app_ula`, `open_app_tun`,
`spawn_encap_pair`, `EncapHandles::abort`).

## Requirements

### Requirement: An app opens a house /128 through mjolnir-app-tun

`mjolnir-app-tun` SHALL compute `app_ula` from a 32-byte house guild id and a 32-byte app public key, using subnet id 1. It SHALL open one Linux TUN at MTU 1280, assign only that `/128`, install only one peer `/128` via that device, and pass one IP packet through `spawn_encap_pair` on the single `DatagramConn` the caller supplies. An unknown peer node id SHALL NOT select another conn. The session SHALL own the device, both encap tasks, and the routes it added. Either encap task returning SHALL abort both tasks through `EncapHandles::abort` and SHALL then remove that address and those routes. Drop SHALL call that same `abort()` even before the supervisor has polled, and SHALL then remove that address and those routes. Dropping a join handle SHALL NOT be treated as cancellation. The crate SHALL NOT link Iroh, read a guild log, assign a covering prefix, or add the address to an Iroh bind set. A non-Linux build SHALL report the computed address with up false and SHALL install nothing.

#### Scenario: Fixture app address

- GIVEN guild id `11` repeated to 32 bytes and app key `ab` repeated to 32 bytes
- WHEN the crate computes the app address
- THEN the address is `fd81:2b22:8aca:1:bd5e:fa7a:5460:697`

#### Scenario: One packet uses the supplied conn

- GIVEN one open session and one `DatagramConn`
- WHEN one IP packet is written toward the peer `/128`
- THEN that conn observes the packet and the crate has not opened an Iroh connection

#### Scenario: A pending receive is aborted when the TUN read returns

- GIVEN one open session whose receive is still pending
- WHEN the TUN read task returns
- THEN `EncapHandles::abort` has cancelled the receive and the owned address and routes are gone

#### Scenario: A pending TUN read is aborted when the receive returns

- GIVEN one open session whose TUN read is still pending
- WHEN the receive task returns
- THEN `EncapHandles::abort` has cancelled the TUN read and the owned address and routes are gone

#### Scenario: Drop aborts both tasks before the supervisor polls

- GIVEN both encap tasks are running and the supervisor has not observed either
- WHEN the session is dropped
- THEN both tasks are aborted and the owned address and routes are gone

#### Scenario: Prefix bytes install a foreign /48

- GIVEN the session was opened for guild `11` repeated and is up
- WHEN the caller passes the 6-byte prefix of guild `22` repeated
- THEN `fd8f:78a9:2caa::/48` is a device route on that TUN and the session's own `/48` is not assigned
