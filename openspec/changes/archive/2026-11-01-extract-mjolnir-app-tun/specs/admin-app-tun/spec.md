## MODIFIED Requirements

### Requirement: Admin has an app /128 on a TUN
<!-- base: 88dcade458b5760351af6505409540d8121f17ca openspec/specs/admin-app-tun/spec.md -->

Lightning Admin SHALL compute its app address as `app_ula` of one house guild id and the actor's Ed25519 public key. The house guild is the one already selected on the existing guild surface when that guild has no outer guild. A workplace guild is such a house. A nested access guild has an outer and SHALL NOT yield an app address. That address SHALL use subnet id 1. Admin SHALL NOT assign a covering prefix. The guild surface SHALL show the computed address when both inputs exist and SHALL NOT open a TUN, add a button, or stage the client SSID.

The runner `open_app_tun` SHALL live in `mjolnir-app-tun`, and Admin SHALL call that crate. The runner SHALL accept that guild id, the actor public key, one peer node id, that peer's app `/128`, and exactly one `DatagramConn` bound to that node id. An unknown node id SHALL NOT select another conn. On Linux the runner SHALL open one TUN at MTU 1280, assign only the local app `/128`, install only that peer `/128` via the TUN, and pass one IP packet through `spawn_encap_pair`. The reply SHALL return through the same conn. The session SHALL own the device, both encap tasks, and the route it added. If installing the peer route fails, the runner SHALL remove the address and close the device. When either encap task returns, the session SHALL abort both encap tasks by calling `EncapHandles::abort`. Dropping those join handles SHALL NOT be the cancellation. The session SHALL then remove the address and the peer route, and set up false while still reporting the computed address. Drop of the session SHALL call that same `abort()` on both encap tasks, including when Drop runs before the supervisor first polls, and SHALL then remove the owned address and routes. A guild or key change SHALL be the caller dropping the session rather than the session retargeting itself. Up SHALL be true only when the assigned `/128` is usable (prefix 128, not tentative, not DAD-failed), the peer `/128` route is installed, and both tasks are still running. Up SHALL NOT mean the peer answered. The address SHALL NOT be added to an Iroh candidate or bind set. A missing guild id, a missing actor key, a failed or unprivileged TUN open, or a non-Linux build SHALL NOT install a route, change Babel, or stage the client SSID. When the inputs exist and the TUN is not usable, the computed address SHALL still be reported with up false. The guild-window blank for a nested access guild SHALL stay in Admin.

#### Scenario: Fixture app address

- GIVEN guild id `11` repeated to 32 bytes and app key `ab` repeated to 32 bytes
- WHEN Admin computes the app address
- THEN the address is `fd81:2b22:8aca:1:bd5e:fa7a:5460:697`

#### Scenario: One packet reaches the peered node id

- GIVEN two TUN devices in different netns, a peer node id, and the conn bound to that id
- WHEN one ICMP echo is written at the local app `/128` toward the peer while `open_app_tun` is running
- THEN that conn observes the echo request and the echo reply, and a conn with another node id observes nothing

#### Scenario: Seam off does not deliver

- GIVEN the peer `/128` route is installed and `spawn_encap_pair` is not running
- WHEN one ICMP echo is sent toward the peer
- THEN the echo is not answered

#### Scenario: Unknown peer installs nothing

- GIVEN a conn bound to a different node id than the one requested
- WHEN `open_app_tun` is called
- THEN up is false, no route is installed, and the other conn receives nothing

#### Scenario: Selected workplace house shows its app address

- GIVEN the selected guild is a workplace house with no outer guild, and an actor key
- WHEN the guild surface looks up the app address
- THEN it shows `app_ula` of that workplace guild and does not open a TUN

#### Scenario: Nested access guild shows no app address

- GIVEN the selected guild is an access guild with an outer guild
- WHEN the guild surface looks up the app address
- THEN no app address is shown and no TUN is opened

#### Scenario: Peer route failure leaves nothing up

- GIVEN the local `/128` can be assigned and the peer route install fails
- WHEN `open_app_tun` returns
- THEN up is false and the local address is not left on the device

#### Scenario: Conn close clears the seam

- GIVEN `open_app_tun` is up and the conn then closes
- WHEN either encap task returns
- THEN both encap tasks have been aborted through `EncapHandles::abort`, the owned address and peer route are removed, and up is false while the computed address remains

#### Scenario: A pending receive is aborted when the TUN read returns

- GIVEN the session is up and the receive task is blocked in `recv_datagram`
- WHEN the TUN read task returns
- THEN the receive task is aborted, the owned address and peer route are removed, and up is false

#### Scenario: A pending TUN read is aborted when the receive returns

- GIVEN the session is up and the TUN read task is blocked
- WHEN the receive task returns
- THEN the TUN read task is aborted, the owned address and peer route are removed, and up is false

#### Scenario: Drop before the supervisor polls aborts both tasks

- GIVEN both encap tasks are running and the supervisor has not yet observed either one
- WHEN the session is dropped
- THEN both encap tasks are aborted and the owned address and peer route are removed

#### Scenario: Missing guild opens nothing

- GIVEN an actor key and no house guild id
- WHEN Admin looks up the app address
- THEN no address is shown and no TUN is opened

#### Scenario: Signing the TUN does not stage the SSID

- GIVEN the TUN packet exchange runs
- WHEN the client network name is inspected
- THEN it is unchanged and no Babel config is written
