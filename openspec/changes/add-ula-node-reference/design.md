# add-ula-node-reference

## Decision

The backhaul reference the operator reads is `router_ula(guild, node_id)`. `10.254` stays on every surface as the transport pin. Nothing in this node dials Iroh or rewrites SSH listen addresses.

The Display form is the string `router_ula` already returns. For the fixture that is `fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df`, not the compressed `fd81:2b22:8aca::ddcd:…` form.

## Surfaces

Directory. `DirectoryNeighbor` gains optional `house_ula`. When `/etc/mjolnir/house-guild-id` is a valid guild file, the value is `router_ula` of that guild and the neighbor's node id. Otherwise the field is omitted. `addrs` is unchanged.

Admin address list. The `/api/node` probe keeps `house_ula`. The discovery row shows it as the reference and shows `mesh_backhaul` as the transport pin. A missing `house_ula` leaves the row on `10.254` alone.

Fleet inventory. The walked column stays `backhaul_addr` (`10.254`). A formatter next to that parser prints `name`, the transport address, and `router_ula` for each member line. It calls the crate function. Pending lines (`-` for the node id) print no `/128`. The inventory header says the printed `/128` is the reference and the column is the pin.

## Reachability

Same-radio and cross-site SSH to that `/128` are the Babel host route from `add-ula-host-forwarding`. This node checks that the shown string equals `router_ula`. It does not run `mesh-errand` and does not deploy.
