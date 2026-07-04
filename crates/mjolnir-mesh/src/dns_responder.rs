//! `.mesh` DNS responder (S1.1, bead mjolnir-mesh-e21.1.1): a minimal
//! authoritative responder for the `.mesh` zone. Binds a UDP socket (default
//! `127.0.0.1:5335`, the port dnsmasq's `server=/mesh/127.0.0.1#5335` stanza
//! forwards `.mesh` queries to — see `docs/sprints/002-mesh-naming/architecture-decisions.md`
//! D-001/D-005) and answers every query with NXDOMAIN + an SOA authority
//! record. Well-known (e21.1.2) and CRDT-projected service (e21.1.3) answers
//! plug in later through the [`NameTable`] seam below — this story only
//! wires the default.
//!
//! Never panics on malformed input: a packet that fails to parse (or a reply
//! that fails to serialize) is logged at debug/warn and dropped — this
//! responder must never take its recv loop down over a bad client.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use simple_dns::rdata::{RData, A, SOA};
use simple_dns::{Name, Packet, PacketFlag, ResourceRecord, CLASS, RCODE};
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

/// Default bind port for the `.mesh` responder (loopback-only — dnsmasq is
/// the only client). Configurable so tests can bind an ephemeral port
/// instead of racing a real 5335 listener on the host.
pub const DEFAULT_DNS_PORT: u16 = 5335;

/// UDP responses are capped at 512 bytes for this story's scope (no EDNS0
/// larger-response negotiation yet) — the classic plain-DNS ceiling.
const MAX_RESPONSE_LEN: usize = 512;

/// Query recv buffer size. Larger than 512 so an EDNS0 OPT-bearing query
/// (which may legally exceed the classic 512B ceiling) is never truncated on
/// the way in; the OPT record itself is tolerated and ignored (see
/// [`handle_query`]).
const RECV_BUF_LEN: usize = 4096;

/// Seam for well-known (e21.1.2) and CRDT-projected service (e21.1.3)
/// answers. Returning `None` falls through to this story's NXDOMAIN+SOA
/// default. `name` is the query name as written on the wire, dotted and
/// lowercase-insensitive per DNS convention (e.g. `"hello.mesh."`).
pub trait NameTable: Send + Sync {
    /// Look up A-record answers for `name`. `None` means "no answer here" —
    /// SRV/TXT lookups will be added as their own methods on this trait when
    /// e21.1.3 lands, rather than overloading this one.
    fn lookup_a(&self, name: &str) -> Option<Vec<Ipv4Addr>>;
}

/// This story's table: every name falls through to NXDOMAIN+SOA. Later
/// stories replace this with a table backed by the CRDT service/user books.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoAnswers;

impl NameTable for NoAnswers {
    fn lookup_a(&self, _name: &str) -> Option<Vec<Ipv4Addr>> {
        None
    }
}

/// A bound, running responder. Dropping this does not stop the background
/// task — call [`ResponderHandle::abort`] at shutdown, as `mjolnir-meshd` does.
pub struct ResponderHandle {
    /// The address actually bound (useful in tests that pass port 0).
    pub local_addr: SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl ResponderHandle {
    /// Stop the responder's recv loop.
    pub fn abort(&self) {
        self.task.abort();
    }
}

/// Bind the responder socket and spawn its recv loop. Returns once the
/// socket is bound (not once the loop exits), so callers can sequence
/// startup — `mjolnir-meshd` binds this BEFORE any UCI/dnsmasq reconcile
/// (FR14), so dnsmasq's `.mesh` upstream is answerable the moment it's
/// configured.
pub async fn start(
    bind_addr: SocketAddr,
    table: Arc<dyn NameTable>,
) -> std::io::Result<ResponderHandle> {
    let socket = UdpSocket::bind(bind_addr).await?;
    let local_addr = socket.local_addr()?;
    info!(%local_addr, "mesh DNS responder bound");
    let task = tokio::spawn(recv_loop(socket, table));
    Ok(ResponderHandle { local_addr, task })
}

async fn recv_loop(socket: UdpSocket, table: Arc<dyn NameTable>) {
    let mut buf = [0u8; RECV_BUF_LEN];
    loop {
        let (len, peer) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => {
                // e.g. an ICMP port-unreachable bounced back from a prior
                // send — not fatal, keep serving other peers.
                warn!("mesh DNS responder: recv error: {e}");
                continue;
            }
        };

        match handle_query(&buf[..len], table.as_ref()) {
            Some(reply) => {
                if let Err(e) = socket.send_to(&reply, peer).await {
                    warn!(%peer, "mesh DNS responder: send error: {e}");
                }
            }
            None => {
                // Malformed packet or an unbuildable reply — never crash the
                // loop over a bad client, just drop and keep serving.
                debug!(%peer, "mesh DNS responder: dropping unparseable/unbuildable packet");
            }
        }
    }
}

/// Parse `query_bytes`, dispatch through `table`, and build the wire-format
/// reply. Returns `None` if the query fails to parse or the reply fails to
/// serialize — callers treat that as "drop, don't respond, keep the loop
/// alive."
fn handle_query(query_bytes: &[u8], table: &dyn NameTable) -> Option<Vec<u8>> {
    let query = match Packet::parse(query_bytes) {
        Ok(p) => p,
        Err(e) => {
            debug!("mesh DNS responder: failed to parse query: {e}");
            return None;
        }
    };

    // EDNS0 OPT is tolerated: `Packet::parse` already lifts any OPT record
    // out of `additional_records` into `query.opt()`. We simply never look
    // at it and never echo one back in the reply — "tolerate and ignore"
    // per this story's scope.
    let mut reply = Packet::new_reply(query.id());
    reply.set_flags(PacketFlag::AUTHORITATIVE_ANSWER);

    match query.questions.into_iter().next() {
        Some(question) => {
            let qname = question.qname.to_string();
            let answers = table.lookup_a(&qname).filter(|a| !a.is_empty()).map(|addrs| {
                addrs
                    .into_iter()
                    .map(|addr| {
                        ResourceRecord::new(
                            question.qname.clone(),
                            CLASS::IN,
                            30,
                            RData::A(A { address: addr.into() }),
                        )
                    })
                    .collect::<Vec<_>>()
            });

            reply.questions.push(question);

            match answers {
                Some(records) => reply.answers = records,
                None => {
                    *reply.rcode_mut() = RCODE::NameError;
                    reply.name_servers.push(mesh_soa_record());
                }
            }
        }
        None => {
            // No question section at all — nothing to look up; still answer
            // NXDOMAIN+SOA so a parseable-but-empty query gets a well-formed,
            // bounded response instead of silence.
            *reply.rcode_mut() = RCODE::NameError;
            reply.name_servers.push(mesh_soa_record());
        }
    }

    let bytes = match reply.build_bytes_vec() {
        Ok(b) => b,
        Err(e) => {
            warn!("mesh DNS responder: failed to build reply: {e}");
            return None;
        }
    };

    if bytes.len() > MAX_RESPONSE_LEN {
        // Not reachable at this story's answer sizes; guarded anyway so a
        // future oversized answer can't silently violate the UDP/512B
        // contract.
        warn!(len = bytes.len(), "mesh DNS responder: reply exceeds 512B, dropping");
        return None;
    }

    Some(bytes)
}

/// The SOA authority record for negative (`NXDOMAIN`/`NODATA`) answers in the
/// `.mesh` zone (decision D-005): owner name is the zone apex (`mesh.`,
/// this responder's authority); TTL matches `minimum` per RFC 2308's
/// negative-caching convention.
fn mesh_soa_record() -> ResourceRecord<'static> {
    ResourceRecord::new(
        Name::new_unchecked("mesh."),
        CLASS::IN,
        30,
        RData::SOA(SOA {
            mname: Name::new_unchecked("hello.mesh."),
            rname: Name::new_unchecked("ops.hello.mesh."),
            serial: 1,
            refresh: 3600,
            retry: 600,
            expire: 86400,
            minimum: 30,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_dns::{QCLASS, TYPE};
    use std::net::{IpAddr, SocketAddr};
    use std::time::Duration;

    fn build_query(name: &str, qtype: TYPE) -> Vec<u8> {
        let mut query = Packet::new_query(0x1234);
        query.questions.push(simple_dns::Question::new(
            Name::new(name).unwrap(),
            qtype.into(),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        query.build_bytes_vec().unwrap()
    }

    #[test]
    fn unknown_name_returns_nxdomain_with_soa() {
        let bytes = build_query("unknown.mesh.", TYPE::A);
        let reply_bytes = handle_query(&bytes, &NoAnswers).expect("should build a reply");

        let reply = Packet::parse(&reply_bytes).expect("reply should parse");
        assert_eq!(reply.rcode(), RCODE::NameError);
        assert_eq!(reply.questions.len(), 1);
        assert_eq!(reply.answers.len(), 0);
        assert_eq!(reply.name_servers.len(), 1);
        match &reply.name_servers[0].rdata {
            RData::SOA(soa) => {
                assert_eq!(soa.mname.to_string(), "hello.mesh");
                assert_eq!(soa.rname.to_string(), "ops.hello.mesh");
                assert_eq!(soa.serial, 1);
                assert_eq!(soa.refresh, 3600);
                assert_eq!(soa.retry, 600);
                assert_eq!(soa.expire, 86400);
                assert_eq!(soa.minimum, 30);
            }
            other => panic!("expected SOA authority record, got {other:?}"),
        }
        assert!(reply_bytes.len() <= MAX_RESPONSE_LEN);
    }

    #[test]
    fn malformed_bytes_never_panics() {
        // Assorted garbage: empty, too short, and a header claiming a
        // question section that isn't actually there (truncated body).
        assert!(handle_query(&[], &NoAnswers).is_none());
        assert!(handle_query(&[0xFF; 3], &NoAnswers).is_none());
        let mut truncated_header = [0u8; 12];
        truncated_header[5] = 1; // QDCOUNT = 1, but no question bytes follow
        assert!(handle_query(&truncated_header, &NoAnswers).is_none());
        assert!(handle_query(&[0xAA; 200], &NoAnswers).is_none());
    }

    #[test]
    fn edns0_opt_is_tolerated_and_ignored() {
        let mut query = Packet::new_query(0x5678);
        query.questions.push(simple_dns::Question::new(
            Name::new("foo.mesh.").unwrap(),
            TYPE::A.into(),
            QCLASS::CLASS(CLASS::IN),
            false,
        ));
        *query.opt_mut() = Some(simple_dns::rdata::OPT {
            opt_codes: Vec::new(),
            udp_packet_size: 4096,
            version: 0,
        });
        let bytes = query.build_bytes_vec().unwrap();

        let reply_bytes = handle_query(&bytes, &NoAnswers).expect("OPT-bearing query should still get a reply");
        let reply = Packet::parse(&reply_bytes).expect("reply should parse");
        assert_eq!(reply.rcode(), RCODE::NameError);
        // We never echo an OPT back — tolerate and ignore, not negotiate.
        assert!(reply.opt().is_none());
    }

    #[test]
    fn empty_question_section_still_gets_a_bounded_reply() {
        let query = Packet::new_query(0x9);
        let bytes = query.build_bytes_vec().unwrap();

        let reply_bytes = handle_query(&bytes, &NoAnswers).expect("should still build a reply");
        let reply = Packet::parse(&reply_bytes).expect("reply should parse");
        assert_eq!(reply.rcode(), RCODE::NameError);
        assert_eq!(reply.name_servers.len(), 1);
    }

    #[tokio::test]
    async fn responder_binds_and_answers_over_the_wire() {
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let handle = start(bind_addr, Arc::new(NoAnswers))
            .await
            .expect("responder should bind an ephemeral port");

        let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        client.connect(handle.local_addr).await.unwrap();

        // A well-formed query gets NXDOMAIN+SOA.
        let query = build_query("hello.mesh.", TYPE::A);
        client.send(&query).await.unwrap();
        let mut buf = [0u8; RECV_BUF_LEN];
        let n = tokio::time::timeout(Duration::from_secs(2), client.recv(&mut buf))
            .await
            .expect("responder should reply before the timeout")
            .unwrap();
        let reply = Packet::parse(&buf[..n]).unwrap();
        assert_eq!(reply.rcode(), RCODE::NameError);

        // A garbage datagram must not kill the loop — the next well-formed
        // query still gets answered.
        client.send(&[0xFF; 5]).await.unwrap();
        client.send(&query).await.unwrap();
        let n = tokio::time::timeout(Duration::from_secs(2), client.recv(&mut buf))
            .await
            .expect("responder should still be alive after a garbage packet")
            .unwrap();
        let reply = Packet::parse(&buf[..n]).unwrap();
        assert_eq!(reply.rcode(), RCODE::NameError);

        handle.abort();
    }
}
