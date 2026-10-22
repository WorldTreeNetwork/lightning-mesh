# Wired node profiles

## Profiles

| Id | Ports | Radios | Cable loss |
|---|---|---|---|
| `ethernet-gateway-satellites` | Gateway: Ethernet WAN. Other nodes: no WAN role required | 5 GHz mesh, 2.4 GHz client AP | Mesh remains the backhaul |
| `wired-core` | Distinct WAN, node-link, and client | Both client bands only if no radio is required for mesh | `isolate` |
| `wired-core-with-fallback` | Distinct WAN, node-link, and client | One radio stays mesh; the spare band is client | `reserve-mesh` |

`ethernet-gateway-satellites` is the default. It matches the current fleet
split (5 GHz mesh id `mjolnir-mesh`, 2.4 GHz client). `wired-core` is
refused for a model that has no qualified port map. Observed evidence, not a
qualification: Cudy AP3000 Outdoor v1 has been seen with `eth0` as WAN and no
client Ethernet on `br-lan`. That observation is not a port map. This change
does not assign AP3000 a wired-core profile.

## Role rules

- A port or radio has one role: port `wan`, `node-link`, `client`, or
  `disabled`; radio `mesh`, `client-ap`, `station`, or `disabled`.
- Client ports do not run babel.
- Node-link ports do not serve resident DHCP.
- WAN is not bridged into `br-lan`.
- Client LANs stay per-node routed prefixes. Profiles do not bridge `br-lan`
  across nodes.
- Profile apply does not add `192.168.1.1/24` to a node that does not already
  use it as recovery, and does not delete an existing house-gateway
  `0.0.0.0/0` babel export as a side effect.

## Apply

A profile change is one `network-change-transactions` plan. Refusal happens
before mutation. Rollback restores the previous profile. This change ships
the profile decider and the plan builder. It does not SSH the fleet or
retune live radios.

Station uplink credentials and candidate tests belong to
`add-wifi-internet-uplink`. A profile may leave a radio role `station` only
when that change's plan assigns it.

## Not owed here

- Claiming a model is qualified. `mjolnir-mesh-lpv` owns hardware trials.
- Guest or resident passphrase Wi-Fi.
- Automatic radio optimization.
