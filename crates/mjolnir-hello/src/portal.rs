// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Captive-portal probe handling — the "front desk finds YOU" path.
//!
//! Client operating systems ask a fixed question on join ("can I reach the
//! internet?") by fetching a well-known URL and comparing the body against an
//! expected constant. Lightning Mesh answers with that success response by
//! default so joining never waits for portal interaction.
//!
//! **This is not a walled garden.** The mesh never blocks traffic. The front
//! desk remains available voluntarily at `http://hello.mesh/`; neither the
//! IdentiKey ceremony nor a dismissal action is required for internet use.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a pass-through lasts before the portal may greet a client again.
/// Long enough that nobody sees the sheet twice in one evening; short enough
/// that a later visit re-offers the IdentiKey ceremony.
const RELEASE_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Clients that took the pass-through: IP -> when they took it. `mjolnir-hello`
/// runs a single-threaded request loop; the `Mutex` matches [`ChallengeStore`]'s
/// convention so the store stays safely shareable if that ever changes.
///
/// [`ChallengeStore`]: crate::routes::ChallengeStore
pub type PortalReleases = Mutex<HashMap<IpAddr, Instant>>;

pub fn new_portal_releases() -> PortalReleases {
    Mutex::new(HashMap::new())
}

/// Record that `ip` took the pass-through. Prunes expired entries on the way
/// in, so the map stays bounded by *currently released* clients rather than by
/// every client the node has ever seen.
pub fn release(store: &PortalReleases, ip: IpAddr) {
    let Ok(mut map) = store.lock() else {
        return;
    };
    let now = Instant::now();
    map.retain(|_, taken| now.duration_since(*taken) < RELEASE_TTL);
    map.insert(ip, now);
}

/// A client-OS connectivity probe, identified by the path it fetches.
///
/// Each variant's [`success`](Probe::success) is the *exact* response that OS
/// treats as "this network is open" — matched byte-for-byte by the OS, so these
/// constants are a wire contract, not cosmetics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// iOS / macOS Captive Network Assistant.
    Apple,
    /// Android (and Chrome OS) — expects a bare `204 No Content`.
    Android,
    /// Windows NCSI text probe.
    Windows,
    /// Windows NCSI secondary probe.
    WindowsNcsi,
    /// Firefox's own portal detection.
    Firefox,
}

impl Probe {
    /// Classify a request path as a known OS probe. Matching is on path only —
    /// meshd has already pointed the probe domains at this node, so anything
    /// arriving at these paths is a probe regardless of Host header.
    pub fn for_path(path: &str) -> Option<Probe> {
        // Strip any query string; Android appends cache-busting params.
        let path = path.split('?').next().unwrap_or(path);
        match path {
            "/hotspot-detect.html" | "/library/test/success.html" => Some(Probe::Apple),
            "/generate_204" | "/gen_204" => Some(Probe::Android),
            "/connecttest.txt" => Some(Probe::Windows),
            "/ncsi.txt" => Some(Probe::WindowsNcsi),
            "/success.txt" | "/canonical.html" => Some(Probe::Firefox),
            _ => None,
        }
    }

    /// The exact `(status, content_type, body)` this OS expects from an open
    /// network. Served to every client by default.
    pub fn success(self) -> (u16, &'static str, &'static [u8]) {
        match self {
            Probe::Apple => (
                200,
                "text/html",
                b"<HTML><HEAD><TITLE>Success</TITLE></HEAD><BODY>Success</BODY></HTML>".as_slice(),
            ),
            Probe::Android => (204, "text/plain", b"".as_slice()),
            Probe::Windows => (200, "text/plain", b"Microsoft Connect Test".as_slice()),
            Probe::WindowsNcsi => (200, "text/plain", b"Microsoft NCSI".as_slice()),
            Probe::Firefox => (200, "text/plain", b"success\n".as_slice()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_each_os_probe() {
        assert_eq!(Probe::for_path("/hotspot-detect.html"), Some(Probe::Apple));
        assert_eq!(Probe::for_path("/generate_204"), Some(Probe::Android));
        assert_eq!(Probe::for_path("/gen_204"), Some(Probe::Android));
        assert_eq!(Probe::for_path("/connecttest.txt"), Some(Probe::Windows));
        assert_eq!(Probe::for_path("/ncsi.txt"), Some(Probe::WindowsNcsi));
        assert_eq!(Probe::for_path("/success.txt"), Some(Probe::Firefox));
        assert_eq!(Probe::for_path("/"), None);
        assert_eq!(Probe::for_path("/api/directory"), None);
    }

    #[test]
    fn ignores_cache_busting_query() {
        assert_eq!(
            Probe::for_path("/generate_204?rand=12345"),
            Some(Probe::Android)
        );
    }

    /// The success payloads are byte-compared by the client OS — a stray
    /// newline or changed case silently breaks pass-through.
    #[test]
    fn success_payloads_are_the_exact_os_constants() {
        assert_eq!(
            Probe::Apple.success(),
            (
                200,
                "text/html",
                b"<HTML><HEAD><TITLE>Success</TITLE></HEAD><BODY>Success</BODY></HTML>".as_slice()
            )
        );
        assert_eq!(
            Probe::Android.success(),
            (204, "text/plain", b"".as_slice())
        );
        assert_eq!(
            Probe::Windows.success(),
            (200, "text/plain", b"Microsoft Connect Test".as_slice())
        );
    }

    #[test]
    fn release_round_trips_per_client() {
        let store = new_portal_releases();
        let a: IpAddr = "10.42.12.50".parse().unwrap();
        let b: IpAddr = "10.42.12.51".parse().unwrap();

        release(&store, a);
        let map = store.lock().unwrap();
        assert!(map.contains_key(&a));
        assert!(!map.contains_key(&b));
    }
}
