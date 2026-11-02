# Tasks

- [x] Add optional `house_ula` on a directory neighbor, computed with `router_ula` when the guild file is valid, omitted otherwise
- [x] Keep `house_ula` from `/api/node` in the Admin probe and show it on the address list as the reference, with `10.254` still visible as the transport pin
- [x] Print the fleet inventory as transport pin plus `router_ula`, without changing the walked `backhaul_addr` column or adding a second hash
- [x] Assert the fixture Display form `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df` for guild `11` repeated and node id `node-example`, and assert a missing guild file omits the field
