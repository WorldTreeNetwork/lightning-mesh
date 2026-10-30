// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Pure certificate-validity classification and the HTTPS directory record.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
