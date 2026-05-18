# Sub-Hypothesis H1a — git-annex native Iroh transport internals

## Summary

The git-annex native Iroh transport (10.20251103) uses iroh **purely as a QUIC tunnel** via `dumbpipe`, carrying git-annex's existing line-based p2p protocol unchanged. **`iroh-blobs` is not involved.** Content is addressed entirely by git-annex SHA256 keys, not BLAKE3. Building a `git-annex-remote-iroh` special remote that exposes iroh-blobs would be a separate, additional piece of work.

## Evidence

**git-annex-p2p-iroh script behavior** [1]: Operates in three modes — `dumbpipe generate-ticket` for peer discovery, `dumbpipe connect` for outbound, `dumbpipe listen-unix` for inbound on a local socket. It is a socket-based transport adapter. git-annex's p2p protocol runs over the socket unchanged; no iroh-blobs interaction.

**git-annex p2p protocol design** [2]: Line-based, references content by **git-annex keys** (`PUT AssociatedFile Key`, `GET Offset AssociatedFile Key`, `DATA <bytes>`). Carried over Tor, SSH, sockets — Iroh is just another socket transport.

**dumbpipe's abstraction level** [3]: Explicitly "netcat-like" — encrypted, hole-punched bidirectional QUIC stream, protocol-agnostic. Raw streams, not content-addressed blobs. The iroh-blobs ALPN is only an optional advanced feature of iroh, not dumbpipe's primary interface.

**iroh-blobs is a separate protocol** [4]: BLAKE3 content addressing over its own QUIC protocol — entirely separate from dumbpipe's stream abstraction.

## Implication for the Architecture Decision

A separate `git-annex-remote-iroh` special remote is still required if the team wants:
- BLAKE3-addressed dedup across peers
- The blob store to be fetchable by any iroh peer (not just paired git-annex repos)
- Integration with other iroh-blobs consumers

The existing native transport alone gives p2p sync between paired git-annex repos and is genuinely useful — but doesn't put asset blobs into iroh-blobs.

## Confidence

**Level**: high. Three independent sources converge.

## Sources

- [1] https://git-annex.branchable.com/special_remotes/p2p/git-annex-p2p-iroh
- [2] https://git-annex.branchable.com/design/p2p_protocol/
- [3] https://github.com/n0-computer/dumbpipe
- [4] https://docs.rs/iroh-blobs/latest/iroh_blobs/
- [5] https://git-annex.branchable.com/tips/peer_to_peer_network_with_iroh/

## Open Questions

- Future git-annex iroh-blobs special remote — not on any visible roadmap.
- distribits.live 2025 talk "Iroh p2p QUIC transport and resumable verified transfers" — unclear if this is dumbpipe tunnel resumability or a separate iroh-blobs integration. Worth a follow-up read if resumability guarantees are load-bearing.
