# Tasks

- [ ] `node_profile.rs`: single-port rule in `decide`; fixture for a node
      whose only port is on `br-lan`
- [ ] Conversion plan: `network.wan`/`wan6` on the port, `br-lan` ports
      empty with `bridge_empty`, firewall and `dhcp.wan` left as shipped
- [ ] `heal.rs`: foreign DHCP server / router on the client bridge
- [ ] Apply to ap3000-outdoor-b through the transaction path
- [ ] Walk test: roam between two gateway nodes, record what a call and a
      TCP flow do

## Handoffs

- Egress policy across two gateways: decision after the walk test.
