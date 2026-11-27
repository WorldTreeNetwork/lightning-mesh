# Tasks

- [ ] `node_profile.rs`: single-port rule in `decide`; fixture for a node
      whose only port is on `br-lan`
- [ ] Conversion plan: `network.wan`/`wan6` on the port, `br-lan` ports
      empty with `bridge_empty`, firewall and `dhcp.wan` left as shipped
- [ ] `heal.rs`: foreign DHCP server / router on the client bridge
- [x] Apply to ap3000-outdoor-b (2026-11-13, one-off commit-confirm script
      in notes/: `mjolnir-txn` only reloads Wi-Fi, so it could not carry a
      port-role change. WAN 10.26.1.105, health gate passed in 5 s)
- [ ] Teach `mjolnir-txn` a network reload so this goes through it
- [ ] Walk test: roam between two gateway nodes, record what a call and a
      TCP flow do

## Handoffs

- Egress policy across two gateways: decision after the walk test.
