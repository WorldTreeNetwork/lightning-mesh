# Tasks

- [ ] Add `crates/mjolnir-app-tun` with the address functions, `DatagramConn`, `spawn_encap_pair`, and the Linux session
- [ ] Make `mjolnir-mesh` and Admin depend on that crate, and remove the session from Admin
- [ ] Sync foreign `/48` routes from prefix bytes the caller passes
- [ ] Keep the Admin guild window, including the nested-access blank, in Admin
- [ ] Move the session tests and keep `cargo test -p mjolnir-mesh` and the Admin lib tests passing
- [ ] Fresh advise after preparation
