# source

# Rayfish firewall

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish firewall](../raw/notes/2026-11-06-rayfish-firewall.md)
> Updated: 2026-11-06

Rayfish inspects IP packets on the TUN. The notes say inbound TCP and UDP are denied, inbound ICMP is allowed, and outbound is allowed. Rules can be per peer and per network. The firewall does not see the QUIC control streams. The draft also said the file was both about 56K lines and about 1,200 lines. Both figures are dropped.

This mesh has no conntrack on `mjolnir0`. The outer nft table is routing policy for the underlay, not that firewall. There is no Rayfish-style exit node. Adding one is a new design, not a missing file in this tree.
