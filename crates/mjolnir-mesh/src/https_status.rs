// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Certificate-validity classification and installed HTTPS directory records.

use serde::{Deserialize, Serialize};
use std::path::Path;

const DAY_SECONDS: u64 = 86_400;
const RENEWAL_DUE_SECONDS: u64 = 10 * DAY_SECONDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpsState {
    Ok,
    Due,
    Failing,
    Expired,
}

/// All times are Unix seconds. Expiry takes precedence over renewal failure;
/// failure takes precedence over the ten-day renewal window.
pub fn https_status(now: u64, not_after: u64, last_renewal_failed: bool) -> HttpsState {
    if now >= not_after {
        HttpsState::Expired
    } else if last_renewal_failed {
        HttpsState::Failing
    } else if not_after - now <= RENEWAL_DUE_SECONDS {
        HttpsState::Due
    } else {
        HttpsState::Ok
    }
}

/// One installed certificate, ready for the additive `https_names` projection.
/// No record is published until the node has certificate metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpsName {
    pub fqdn: String,
    /// Certificate expiry as Unix seconds, not milliseconds.
    pub not_after: u64,
    pub state: HttpsState,
}

impl HttpsName {
    pub fn new(fqdn: String, not_after: u64, now: u64, last_renewal_failed: bool) -> Self {
        Self {
            fqdn,
            not_after,
            state: https_status(now, not_after, last_renewal_failed),
        }
    }
}

#[derive(Deserialize)]
struct InstalledCertificate {
    fqdn: String,
    not_after: u64,
    last_renewal_failed: bool,
}

/// Read installed certificate metadata beneath the system root. Missing,
/// unreadable, empty or invalid JSON files yield no names; malformed records
/// are skipped. States are classified using `now` in Unix seconds on every read.
pub fn load_https_names(system_root: &Path, now: u64) -> Vec<HttpsName> {
    let path = system_root.join("var/run/mjolnir/https-names.json");
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let Ok(records) = serde_json::from_slice::<Vec<serde_json::Value>>(&bytes) else {
        return Vec::new();
    };
    records
        .into_iter()
        .filter_map(|record| serde_json::from_value::<InstalledCertificate>(record).ok())
        .map(|record| {
            HttpsName::new(
                record.fqdn,
                record.not_after,
                now,
                record.last_renewal_failed,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_status_loader_classifies_installed_records_at_read_time() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("var/run/mjolnir/https-names.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let now = 1_800_000_000;
        let records = serde_json::json!([
            {"fqdn": "ok.example", "not_after": now + RENEWAL_DUE_SECONDS + 1, "last_renewal_failed": false},
            {"fqdn": "due.example", "not_after": now + RENEWAL_DUE_SECONDS, "last_renewal_failed": false},
            {"fqdn": "failing.example", "not_after": now + 1, "last_renewal_failed": true},
            {"fqdn": "expired.example", "not_after": now, "last_renewal_failed": true},
            {"fqdn": "missing-fields.example"},
            {"fqdn": "bad-expiry.example", "not_after": "tomorrow", "last_renewal_failed": false},
            null
        ]);
        std::fs::write(&path, serde_json::to_vec(&records).unwrap()).unwrap();

        let names = load_https_names(root.path(), now);
        assert_eq!(names.len(), 4);
        assert_eq!(names[0].fqdn, "ok.example");
        assert_eq!(names[0].not_after, now + RENEWAL_DUE_SECONDS + 1);
        assert_eq!(names[0].state, HttpsState::Ok);
        assert_eq!(names[1].state, HttpsState::Due);
        assert_eq!(names[2].state, HttpsState::Failing);
        assert_eq!(names[3].state, HttpsState::Expired);

        let later = load_https_names(root.path(), now + 1);
        assert_eq!(later[0].state, HttpsState::Due);
        assert_eq!(later[2].state, HttpsState::Expired);
    }

    #[test]
    fn https_status_loader_tolerates_missing_empty_and_invalid_files() {
        let root = tempfile::tempdir().unwrap();
        assert!(load_https_names(root.path(), 100).is_empty());
        let path = root.path().join("var/run/mjolnir/https-names.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for contents in ["", " ", "not json", "[", "{}", "null", "[]"] {
            std::fs::write(&path, contents).unwrap();
            assert!(
                load_https_names(root.path(), 100).is_empty(),
                "{contents:?}"
            );
        }
    }

    #[test]
    fn https_status_expiry_wins_at_and_after_not_after() {
        for failed in [false, true] {
            assert_eq!(https_status(100, 100, failed), HttpsState::Expired);
            assert_eq!(https_status(101, 100, failed), HttpsState::Expired);
        }
    }

    #[test]
    fn https_status_failure_wins_while_certificate_is_live() {
        for remaining in [1, RENEWAL_DUE_SECONDS, RENEWAL_DUE_SECONDS + 1] {
            assert_eq!(
                https_status(100, 100 + remaining, true),
                HttpsState::Failing
            );
        }
    }

    #[test]
    fn https_status_due_window_includes_exactly_ten_days() {
        for remaining in [1, RENEWAL_DUE_SECONDS - 1, RENEWAL_DUE_SECONDS] {
            assert_eq!(https_status(100, 100 + remaining, false), HttpsState::Due);
        }
        assert_eq!(
            https_status(100, 100 + RENEWAL_DUE_SECONDS + 1, false),
            HttpsState::Ok
        );
    }

    #[test]
    fn https_status_directory_record_uses_unix_seconds_and_lowercase_states() {
        for (remaining, failed, state) in [
            (RENEWAL_DUE_SECONDS + 1, false, "ok"),
            (RENEWAL_DUE_SECONDS, false, "due"),
            (1, true, "failing"),
            (0, true, "expired"),
        ] {
            let name = HttpsName::new(
                "n-example.house.mesh.worldtree.network".into(),
                100 + remaining,
                100,
                failed,
            );
            let json = serde_json::to_value(&name).unwrap();
            assert_eq!(json["not_after"], 100 + remaining);
            assert_eq!(json["state"], state);
            assert_eq!(serde_json::from_value::<HttpsName>(json).unwrap(), name);
        }
    }
}
