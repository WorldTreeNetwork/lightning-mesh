# source

# Rayfish DNS

> Sources: Clay's Rayfish notes, 2026-11-06
> Raw: [Rayfish DNS](../raw/notes/2026-11-06-rayfish-dns.md)
> Updated: 2026-11-06

Rayfish serves `.ray` from the daemon and can forward through DNS-over-TLS. The draft's "45K lines" figure is omitted.

Mesh DNS is the `.mesh` zone on the node, fed by the directory books. hello.mesh is the visitor name on that node's LAN gateway. Borrowing DoT forwarding or a richer resolver would be new work. It is not a hole in the `.mesh` records this tree already serves.
