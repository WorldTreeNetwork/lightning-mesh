# Tasks

- [x] Bind and dial the identity ULA with no guild and with a different guild
- [x] Copy the identity `/64` into the outer table
- [x] Lamp follows the identity ULA
- [x] Unit tests for the binding, the on-link route, and the dial gate
- [x] Best-effort dial and listen: identity ULA always, plus `router_ula` for each guild id this node has learned. A peer on an older bind answers without running this commit.
- [ ] `mjolnir-mesh-z7gl`: tr3000, wr3000s-bench, and ap3000-outdoor-b answer tr3000-b. Install only a binary that tries that address set. No `--wireless`. Cable is `root@10.42.149.1`.
