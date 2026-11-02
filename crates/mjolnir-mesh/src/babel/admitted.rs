//! Trusted publisher projection and Babel application state.

use std::io;
use std::net::Ipv6Addr;
use std::path::Path;

use crate::tun::{HOUSE_GUILD_PATH, house_ula_prefix, parse_house_guild_id, ula_prefix};

pub const ADMITTED_PREFIXES_PATH: &str = "/var/lib/mjolnir/admitted-prefixes";

/// Parse foreign fd /48 network addresses. This validates shape, not admission.
pub fn parse_admitted_prefixes(text: &str, guild: Option<&str>) -> Vec<Ipv6Addr> {
    let house = guild
        .and_then(parse_house_guild_id)
        .map(|id| house_ula_prefix(&id).addr());
    let legacy = ula_prefix().addr();
    let mut prefixes: Vec<_> = text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let addr: Ipv6Addr = line.strip_suffix("/48").unwrap_or(line).parse().ok()?;
            let bytes = addr.octets();
            (bytes[0] == 0xfd
                && bytes[6..].iter().all(|b| *b == 0)
                && addr != legacy
                && Some(addr) != house)
                .then_some(addr)
        })
        .collect();
    prefixes.sort_by_cached_key(Ipv6Addr::to_string);
    prefixes.dedup();
    prefixes
}

/// Retain the unfiltered last successful read so a guild change is honored even
/// during projection I/O failures. No successful read means no safe update yet.
#[derive(Debug, Default)]
pub struct AdmittedPrefixes {
    last_successful: Option<String>,
}

impl AdmittedPrefixes {
    pub fn read_admitted_prefixes(&mut self) -> Option<Vec<Ipv6Addr>> {
        self.read(
            Path::new(ADMITTED_PREFIXES_PATH),
            Path::new(HOUSE_GUILD_PATH),
        )
    }

    pub fn read(&mut self, projection: &Path, guild_path: &Path) -> Option<Vec<Ipv6Addr>> {
        match std::fs::read_to_string(projection) {
            Ok(text) => self.last_successful = Some(text),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.last_successful = Some(String::new());
            }
            Err(e) => {
                tracing::warn!(path = %projection.display(), "keeping last admitted projection: {e}")
            }
        }
        let guild = std::fs::read_to_string(guild_path).ok();
        self.last_successful
            .as_deref()
            .map(|text| parse_admitted_prefixes(text, guild.as_deref()))
    }
}

/// A successful config write is not a successful Babel application. Call the
/// decision after each successful write (including a no-op), after any debounce.
#[derive(Debug, Default)]
pub struct PendingBabelRestart {
    pending: bool,
}

impl PendingBabelRestart {
    pub fn needs_restart(&mut self, started: bool, changed: bool) -> bool {
        self.pending |= !started || changed;
        self.pending
    }

    /// Only start/restart success acknowledges the on-disk configuration.
    pub fn record_result(&mut self, success: bool) {
        if success {
            self.pending = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::babel::{
        BabelConfigInputs, OverlayRtt, render_babeld_conf, render_overlay_babeld_conf,
        write_atomic_if_changed,
    };

    const HOUSE: &str = "fd81:2b22:8aca::";
    const FOREIGN: &str = "fd8f:78a9:2caa::";
    const ROUTER: &str = "fd81:2b22:8aca:0:ddcd:86a6:64e9:f2df";

    fn renders(router: Option<Ipv6Addr>, prefixes: &[Ipv6Addr], gateway: bool) -> [String; 2] {
        [
            render_babeld_conf(
                &BabelConfigInputs::new(None, &[])
                    .house_router(router)
                    .admitted(prefixes)
                    .gateway(gateway),
            ),
            render_overlay_babeld_conf(
                "mjolnir0",
                None,
                OverlayRtt::default(),
                gateway,
                router,
                prefixes,
            ),
        ]
    }

    fn assert_allows(conf: &str, prefix: &str, allowed: bool) {
        for direction in ["in", "out"] {
            let line = format!("{direction} ip {prefix}/48 eq 128 allow");
            assert_eq!(
                conf.lines().filter(|l| *l == line).count(),
                usize::from(allowed),
                "{conf}"
            );
            if allowed {
                for deny in ["in ip fc00::/7 deny", "out ip fc00::/7 deny"] {
                    assert!(
                        conf.find(&line).unwrap() < conf.find(deny).unwrap(),
                        "{conf}"
                    );
                }
            }
        }
    }

    #[test]
    fn admitted_both_renders_export_house_host_and_allow_foreign_hosts() {
        let router = crate::tun::router_ula(&[0x11; 32], "node-example");
        assert_eq!(router.to_string(), ROUTER);
        let prefixes = parse_admitted_prefixes(FOREIGN, Some(&"11".repeat(32)));
        for gateway in [true, false] {
            for conf in renders(Some(router), &prefixes, gateway) {
                let export = format!("redistribute local ip {ROUTER}/128 allow");
                assert!(conf.contains(&export), "{conf}");
                assert!(conf.find(&export) < conf.find("redistribute local deny"));
                assert_allows(&conf, HOUSE, true);
                assert_allows(&conf, FOREIGN, true);
                assert!(
                    conf.find(&format!("out ip {HOUSE}")) < conf.find(&format!("in ip {FOREIGN}"))
                );
                assert!(!conf.contains("fd12:3456:789a"), "{conf}");
                assert!(!conf.contains("fd53:6213:4797"), "{conf}");
                for line in conf.lines().filter(|l| l.contains(FOREIGN)) {
                    assert!(!line.starts_with("redistribute"), "{line}");
                    assert!(!line.split_whitespace().any(|w| w == "ge" || w == "le"));
                }
                assert_eq!(
                    conf.contains("redistribute ip 0.0.0.0/0 le 0 metric 128"),
                    gateway
                );
                assert_eq!(conf.contains("le 0"), gateway);
            }
        }
    }

    #[test]
    fn admitted_direct_stock_prefix_input_emits_allow() {
        let stock = "fd12:3456:789a::";
        let prefixes = [stock.parse().unwrap()];
        assert_eq!(
            parse_admitted_prefixes(&format!("{stock}/48"), None),
            prefixes
        );
        for conf in renders(None, &prefixes, false) {
            assert_allows(&conf, stock, true);
        }
    }

    #[test]
    fn admitted_parser_counts_only_fd_networks_deduplicated_and_text_sorted() {
        let text = "\n # comment\n fd8f:78a9:2caa::/48 \nFD8F:78A9:2CAA:0:0:0:0:0\n\
                    fd00:2::\nfd00:10::/48\nfc8f:78a9:2caa::/48\nfe80::/48\n2001:db8::/48\n\
                    fd8f:78a9:2caa::1\nfd8f:78a9:2caa:1::/48\nfd8f:78a9:2caa::/64\n\
                    fd8f:78a9:2caa::/128\nfd8f:78a9:2caa::/47\ngarbage\n";
        let expected: Vec<Ipv6Addr> = ["fd00:10::", "fd00:2::", FOREIGN]
            .map(|s| s.parse().unwrap())
            .to_vec();
        let prefixes = parse_admitted_prefixes(text, None);
        assert_eq!(prefixes, expected);
        let equivalent = format!("{FOREIGN}\nfd00:2::/48\nfd00:10::\n{FOREIGN}/48\n");
        assert_eq!(prefixes, parse_admitted_prefixes(&equivalent, None));
        assert_eq!(
            renders(None, &prefixes, false),
            renders(None, &parse_admitted_prefixes(&equivalent, None), false)
        );
    }

    #[test]
    fn admitted_compiled_legacy_prefix_does_not_count() {
        assert_eq!(ula_prefix().addr().to_string(), "fd53:6213:4797::");
        assert!(parse_admitted_prefixes("fd53:6213:4797::/48", None).is_empty());
    }

    #[test]
    fn admitted_configured_house_excluded_even_without_usable_router() {
        let tmp = tempfile::tempdir().unwrap();
        let guild = tmp.path().join("guild");
        let projection = tmp.path().join("prefixes");
        std::fs::write(&guild, "11".repeat(32)).unwrap();
        std::fs::write(&projection, format!("{HOUSE}/48\n{FOREIGN}/48\n")).unwrap();
        let mut reader = AdmittedPrefixes::default();
        let prefixes = reader.read(&projection, &guild).unwrap();
        let router =
            crate::tun::published_house_ula("node-example", Some(&"11".repeat(32)), "", "br-mesh");
        assert_eq!(router, None);
        for conf in renders(router, &prefixes, false) {
            assert_allows(&conf, HOUSE, false);
            assert_allows(&conf, FOREIGN, true);
            assert!(!conf.contains("redistribute local ip fd"));
        }
    }

    #[test]
    fn admitted_missing_or_malformed_guild_excludes_no_house_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let guild = tmp.path().join("guild");
        let projection = tmp.path().join("prefixes");
        std::fs::write(&projection, format!("{HOUSE}/48\n{FOREIGN}/48\n")).unwrap();
        let mut reader = AdmittedPrefixes::default();
        for content in [None, Some("malformed"), Some("")] {
            if let Some(content) = content {
                std::fs::write(&guild, content).unwrap();
            }
            for conf in renders(None, &reader.read(&projection, &guild).unwrap(), false) {
                assert_allows(&conf, HOUSE, true);
                assert_allows(&conf, FOREIGN, true);
                assert!(!conf.contains("redistribute local ip fd"));
            }
        }
    }

    #[test]
    fn admitted_guild_change_rechecks_exclusion_including_cached_projection() {
        let tmp = tempfile::tempdir().unwrap();
        let guild = tmp.path().join("guild");
        let projection = tmp.path().join("prefixes");
        std::fs::write(&projection, format!("{HOUSE}/48\n{FOREIGN}/48\n")).unwrap();
        let mut reader = AdmittedPrefixes::default();
        std::fs::write(&guild, "11".repeat(32)).unwrap();
        assert_eq!(
            reader.read(&projection, &guild).unwrap(),
            [FOREIGN.parse::<Ipv6Addr>().unwrap()]
        );
        for source in [&projection, &tmp.path().to_path_buf()] {
            std::fs::write(&guild, "22".repeat(32)).unwrap();
            for conf in renders(None, &reader.read(source, &guild).unwrap(), false) {
                assert_allows(&conf, HOUSE, true);
                assert_allows(&conf, FOREIGN, false);
                assert!(!conf.contains("redistribute local ip fd"));
            }
        }
    }

    #[test]
    fn admitted_unreadable_projection_keeps_last_good_and_initial_error_skips_update() {
        let tmp = tempfile::tempdir().unwrap();
        let guild = tmp.path().join("guild");
        let projection = tmp.path().join("prefixes");
        let mut reader = AdmittedPrefixes::default();
        // A directory is a deterministic non-ENOENT error, even as root.
        assert_eq!(reader.read(tmp.path(), &guild), None);
        std::fs::write(&projection, FOREIGN).unwrap();
        let good = reader.read(&projection, &guild).unwrap();
        assert_eq!(reader.read(tmp.path(), &guild), Some(good.clone()));
        std::fs::write(&projection, [0xff]).unwrap();
        assert_eq!(reader.read(&projection, &guild), Some(good.clone()));
        for conf in renders(None, &good, false) {
            assert_allows(&conf, FOREIGN, true);
        }
        // ENOENT really removes the last good projection, unlike other errors.
        let removed = reader.read(&tmp.path().join("missing"), &guild).unwrap();
        assert!(removed.is_empty());
        assert_eq!(reader.read(tmp.path(), &guild), Some(vec![]));
    }

    #[test]
    fn admitted_failed_restart_then_unchanged_inputs_retries_and_applies_removal() {
        let tmp = tempfile::tempdir().unwrap();
        let projection = tmp.path().join("prefixes");
        let guild = tmp.path().join("guild");
        let router = Some(ROUTER.parse().unwrap());
        for remove_file in [false, true] {
            std::fs::write(&projection, FOREIGN).unwrap();
            let mut reader = AdmittedPrefixes::default();
            let old = renders(router, &reader.read(&projection, &guild).unwrap(), false);
            if remove_file {
                std::fs::remove_file(&projection).unwrap();
            } else {
                std::fs::write(&projection, "").unwrap();
            }
            let desired = renders(router, &reader.read(&projection, &guild).unwrap(), false);
            for (i, (old, desired)) in old.into_iter().zip(desired).enumerate() {
                let path = tmp.path().join(format!("babel-{i}.conf"));
                std::fs::write(&path, &old).unwrap();
                let mut running = old;
                let mut restart = PendingBabelRestart::default();
                assert!(!restart.needs_restart(true, false));
                assert!(write_atomic_if_changed(&path, &desired).unwrap());
                assert!(restart.needs_restart(true, true));
                restart.record_result(false);
                assert_allows(&running, FOREIGN, true);
                let next = renders(router, &reader.read(&projection, &guild).unwrap(), false);
                assert_eq!(next[i], desired);
                let changed = write_atomic_if_changed(&path, &next[i]).unwrap();
                assert!(!changed);
                assert!(restart.needs_restart(true, changed));
                restart.record_result(true);
                running = std::fs::read_to_string(&path).unwrap();
                assert_allows(&running, FOREIGN, false);
                assert!(running.contains("in ip fc00::/7 deny"));
                assert!(running.contains("out ip fc00::/7 deny"));
                assert!(!restart.needs_restart(true, false));
            }
        }
    }

    #[test]
    fn admitted_failed_start_retries_unchanged_disk_until_success() {
        let mut restart = PendingBabelRestart::default();
        assert!(restart.needs_restart(false, false));
        restart.record_result(false);
        assert!(restart.needs_restart(false, false));
        restart.record_result(true);
        assert!(!restart.needs_restart(true, false));
    }
}
