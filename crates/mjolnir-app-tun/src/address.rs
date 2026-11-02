use std::net::Ipv6Addr;

/// Tiny helper so tests can format the `/48` without depending on `ipnet` here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6NetPrefix {
    pub octets: [u8; 16],
}

impl Ipv6NetPrefix {
    pub fn addr(self) -> Ipv6Addr {
        Ipv6Addr::from(self.octets)
    }
}

/// Domain-separated input for a house `/48` (add-house-ula-prefix).
/// Twenty ASCII bytes, no trailing NUL. Not the legacy mesh prefix and not the
/// CRDT gossip topic: changing either of those must not renumber the house.
pub const HOUSE_ULA_DOMAIN: &[u8] = b"mjolnir/house/ula/v1";

/// Router hosts sit in subnet 0. App hosts sit in subnet 1.
pub const HOUSE_ROUTER_SUBNET: u16 = 0;
pub const HOUSE_APP_SUBNET: u16 = 1;

/// `IFA_F_TENTATIVE`. A tentative address is not usable.
pub const IFA_F_TENTATIVE: u32 = 0x40;
/// `IFA_F_DADFAILED`. A DAD-failed address is not usable.
pub const IFA_F_DADFAILED: u32 = 0x08;

/// A house `/128` is usable when it is present at prefix length 128 and is
/// neither tentative nor DAD-failed. Command status is not part of this.
pub fn host_route_usable(prefix_len: u8, flags: u32) -> bool {
    prefix_len == 128 && flags & IFA_F_TENTATIVE == 0 && flags & IFA_F_DADFAILED == 0
}

fn trim_ascii_ws(text: &str) -> &str {
    text.trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000b}' | '\u{000c}'))
}

/// Accept a guild file body: ASCII trim, then exactly 64 hex digits.
/// Display-form base58 and anything else are absent.
pub fn parse_house_guild_id(text: &str) -> Option<[u8; 32]> {
    let text = trim_ascii_ws(text);
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn iid64(input: &[u8]) -> [u8; 8] {
    let hashed = blake3::hash(input);
    let mut iid = u64::from_be_bytes(hashed.as_bytes()[..8].try_into().expect("8 bytes"));
    if iid == 0 {
        iid = 1;
    }
    iid.to_be_bytes()
}

fn house_host(guild_id: &[u8; 32], subnet: u16, iid: [u8; 8]) -> Ipv6Addr {
    let digest = blake3::hash(&[HOUSE_ULA_DOMAIN, guild_id].concat());
    let prefix = digest.as_bytes();
    let sub = subnet.to_be_bytes();
    Ipv6Addr::from([
        0xfd, prefix[0], prefix[1], prefix[2], prefix[3], prefix[4], sub[0], sub[1], iid[0],
        iid[1], iid[2], iid[3], iid[4], iid[5], iid[6], iid[7],
    ])
}

/// House `/48`: `fd` plus the first five bytes of `blake3(domain || guild_id)`.
pub fn house_ula_prefix(guild_id: &[u8; 32]) -> Ipv6NetPrefix {
    let digest = blake3::hash(&[HOUSE_ULA_DOMAIN, guild_id].concat());
    let b = digest.as_bytes();
    Ipv6NetPrefix {
        octets: [
            0xfd, b[0], b[1], b[2], b[3], b[4], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    }
}

/// Router `/128` in subnet 0. The interface id is `blake3` of the node id's
/// UTF-8, the same input as the legacy mesh address, clamped off 0.
pub fn router_ula(guild_id: &[u8; 32], node_id: &str) -> Ipv6Addr {
    house_host(guild_id, HOUSE_ROUTER_SUBNET, iid64(node_id.as_bytes()))
}

/// App `/128` in subnet 1. The interface id is `blake3` of the raw 32-byte key.
pub fn app_ula(guild_id: &[u8; 32], app_key: &[u8; 32]) -> Ipv6Addr {
    house_host(guild_id, HOUSE_APP_SUBNET, iid64(app_key))
}

/// Desired router `/128` from a guild file body, or none when the file is absent
/// or malformed.
pub fn desired_router_ula(guild_text: Option<&str>, node_id: &str) -> Option<Ipv6Addr> {
    let guild_id = parse_house_guild_id(guild_text?)?;
    Some(router_ula(&guild_id, node_id))
}

/// One IPv6 row from `/proc/net/if_inet6`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfInet6Addr {
    pub addr: Ipv6Addr,
    pub prefix_len: u8,
    pub flags: u32,
    pub iface: String,
}

/// Parse `/proc/net/if_inet6`. Prefix length, scope, and flags are hex.
pub fn parse_if_inet6(body: &str) -> Vec<IfInet6Addr> {
    let mut out = Vec::new();
    for line in body.lines() {
        let mut parts = line.split_whitespace();
        let Some(hex) = parts.next() else { continue };
        let Some(_idx) = parts.next() else { continue };
        let Some(plen) = parts.next() else { continue };
        let Some(_scope) = parts.next() else { continue };
        let Some(flags) = parts.next() else { continue };
        let Some(iface) = parts.next() else { continue };
        if hex.len() != 32 {
            continue;
        }
        let Ok(prefix_len) = u8::from_str_radix(plen, 16) else {
            continue;
        };
        let Ok(flags) = u32::from_str_radix(flags, 16) else {
            continue;
        };
        let mut bytes = [0u8; 16];
        let mut ok = true;
        for (i, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
            match std::str::from_utf8(chunk)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
            {
                Some(b) => bytes[i] = b,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        out.push(IfInet6Addr {
            addr: Ipv6Addr::from(bytes),
            prefix_len,
            flags,
            iface: iface.to_string(),
        });
    }
    out
}

/// Whether `want` is a usable `/128` on `iface` in a `/proc/net/if_inet6` body.
pub fn house_addr_usable(body: &str, iface: &str, want: Ipv6Addr) -> bool {
    parse_if_inet6(body).into_iter().any(|row| {
        row.iface == iface && row.addr == want && host_route_usable(row.prefix_len, row.flags)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_and_app_fixture_vectors() {
        assert_eq!(
            house_ula_prefix(&[0x11; 32]).addr().to_string(),
            "fd81:2b22:8aca::"
        );
        assert_eq!(
            house_ula_prefix(&[0x22; 32]).addr().to_string(),
            "fd8f:78a9:2caa::"
        );
        assert_eq!(
            app_ula(&[0x11; 32], &[0xab; 32]).to_string(),
            "fd81:2b22:8aca:1:bd5e:fa7a:5460:697"
        );
        assert_eq!(
            app_ula(&[0x11; 32], &[0xcd; 32]).to_string(),
            "fd81:2b22:8aca:1:ef30:9577:4565:e614"
        );
        assert_eq!(
            router_ula(&[0x11; 32], "node-example").to_string(),
            "fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df"
        );
        assert_eq!(
            desired_router_ula(Some(&"11".repeat(32)), "node-example"),
            Some(router_ula(&[0x11; 32], "node-example"))
        );
        assert_eq!(desired_router_ula(Some("not hex"), "node-example"), None);
    }
}
