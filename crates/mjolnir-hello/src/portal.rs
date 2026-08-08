// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Captive-portal probe handling — the "front desk finds YOU" path.
//!
//! The discovery problem this solves: people join the mesh SSID and then have
//! no idea `hello.mesh` exists. Every modern client OS already asks a fixed
//! question on join ("can I reach the internet?") by fetching a well-known URL
//! and comparing the body against an expected constant. Answer that question
//! with anything else and the OS opens its captive-portal sheet — which is the
//! one UI surface a phone will show a stranger without being asked.
//!
//! **This is not a walled garden.** The mesh never blocks traffic; meshd only
//! points the probe DOMAINS at this node's front desk (dnsmasq `address=`), and
//! this module answers them. The portal page carries an explicit pass-through
//! ("take me to the internet" / "no thanks"), and once a client takes it, this
//! module serves that client the genuine success payload forever after, so the
//! OS marks the network connected and stops asking. The portal's actual purpose
//! is to offer the IdentiKey ceremony — an opt-in with a deliberately high bar
//! (open the page, type your name, generate a key) — not to gate anything.

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

/// True when `ip` has an unexpired pass-through. A client with no address
/// (`None`) is never treated as released — it gets the portal, which is the
/// safe direction: worst case the sheet shows once more.
pub fn is_released(store: &PortalReleases, ip: Option<IpAddr>) -> bool {
    let Some(ip) = ip else {
        return false;
    };
    let Ok(map) = store.lock() else {
        return false;
    };
    map.get(&ip)
        .is_some_and(|taken| Instant::now().duration_since(*taken) < RELEASE_TTL)
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
    /// network. Served only to clients that took the pass-through.
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

/// The portal page, served to a not-yet-released client for ANY probe path.
///
/// Deliberately self-contained (inline CSS/JS, no bundle, no external fetches):
/// the OS renders this inside a stripped-down mini-browser with no cache and,
/// on a mesh with no uplink, no route to any CDN. It must survive on its own.
pub const PORTAL_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Lightning Mesh</title>
<style>
  :root {
    --bg: #f7f7f5; --fg: #16161a; --muted: #6b6b76;
    --card: #ffffff; --line: #e4e4e0; --accent: #2f6f4f;
    --accent-fg: #ffffff;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #121214; --fg: #f2f2f0; --muted: #9a9aa4;
      --card: #1c1c20; --line: #2c2c32; --accent: #4f9e73;
      --accent-fg: #0d0d0f;
    }
  }
  * { box-sizing: border-box; }
  body {
    margin: 0; background: var(--bg); color: var(--fg);
    font: 16px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    display: flex; align-items: center; justify-content: center;
    min-height: 100vh; padding: 24px;
  }
  .card {
    background: var(--card); border: 1px solid var(--line); border-radius: 14px;
    padding: 28px 24px; max-width: 380px; width: 100%;
  }
  .mark { font-size: 28px; line-height: 1; margin-bottom: 14px; }
  h1 { font-size: 21px; margin: 0 0 8px; letter-spacing: -0.01em; }
  p { margin: 0 0 16px; color: var(--muted); font-size: 14.5px; }
  a.btn, button {
    display: block; width: 100%; text-align: center; cursor: pointer;
    font: inherit; font-weight: 600; font-size: 15px;
    padding: 13px 16px; border-radius: 10px; border: 1px solid var(--line);
    background: transparent; color: var(--fg); text-decoration: none;
    margin-top: 9px;
  }
  a.primary {
    background: var(--accent); color: var(--accent-fg); border-color: var(--accent);
  }
  .sub { font-size: 12.5px; color: var(--muted); margin: 18px 0 0; text-align: center; }
  .hide { display: none; }
</style>
</head>
<body>
  <div class="card">
    <div id="offer">
      <div class="mark">&#9889;</div>
      <h1>You&rsquo;re on Lightning Mesh</h1>
      <p>A local network that belongs to the people in this room &mdash; no
         accounts, no tracking, no company in the middle.</p>
      <p>Make an <strong>IdentiKey</strong>: a cryptographic identity you hold,
         that works here even with the internet unplugged.</p>
      <a class="btn primary" href="http://hello.mesh/">Create your IdentiKey</a>
      <button id="pass" type="button">Take me to the internet</button>
      <button id="nope" type="button">No thanks</button>
      <p class="sub">Nothing is blocked either way. You can come back any time
         at <strong>hello.mesh</strong>.</p>
    </div>
    <div id="done" class="hide">
      <div class="mark">&#10003;</div>
      <h1>You&rsquo;re all set</h1>
      <p>This network won&rsquo;t interrupt you again. Open
         <strong>hello.mesh</strong> whenever you want your IdentiKey.</p>
    </div>
  </div>
<script>
(function () {
  function passThrough() {
    var xhr = new XMLHttpRequest();
    xhr.open('POST', '/api/portal/pass', true);
    xhr.onloadend = function () {
      document.getElementById('offer').className = 'hide';
      document.getElementById('done').className = '';
      // Re-run the OS probe so the sheet notices we're open and closes itself.
      setTimeout(function () {
        location.href = 'http://captive.apple.com/hotspot-detect.html';
      }, 900);
    };
    xhr.send('');
  }
  document.getElementById('pass').onclick = passThrough;
  document.getElementById('nope').onclick = passThrough;
})();
</script>
</body>
</html>
"##;

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

        assert!(!is_released(&store, Some(a)));
        release(&store, a);
        assert!(is_released(&store, Some(a)));
        // Releasing one client must not release the node's other clients.
        assert!(!is_released(&store, Some(b)));
    }

    #[test]
    fn unknown_client_address_is_never_released() {
        let store = new_portal_releases();
        assert!(!is_released(&store, None));
    }

    /// The page has to stand alone inside the OS mini-browser: no external
    /// origins to fetch from, and both escape hatches present.
    #[test]
    fn portal_page_is_self_contained_and_offers_the_pass_through() {
        assert!(!PORTAL_HTML.contains("//cdn"));
        assert!(!PORTAL_HTML.contains("https://"));
        assert!(PORTAL_HTML.contains("/api/portal/pass"));
        assert!(PORTAL_HTML.contains("Take me to the internet"));
        assert!(PORTAL_HTML.contains("No thanks"));
        assert!(PORTAL_HTML.contains("http://hello.mesh/"));
    }
}
