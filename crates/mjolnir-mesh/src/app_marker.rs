// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors
// Lightning Mesh is dual-licensed (AGPL-3.0-or-later or commercial); see LICENSE
// and COMMERCIAL-LICENSE.md at the repository root.

//! Mini-app marker rules shared by name claims and `POST /v0/publish`.
//!
//! Same rules as `hello-mesh-web/src/lib/miniapp/contract.ts` `appPath`:
//! version `1`, path starts with `/`, no `//`, `\`, scheme, or control
//! characters. An omitted path is `/`. No length ceiling. Publish-time
//! checks are feedback; consumers still re-validate.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Stored marker. Omitted `path` on the wire deserializes as `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppMarker {
    pub v: u8,
    #[serde(default = "default_app_path")]
    pub path: String,
}

fn default_app_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMarkerError {
    UnsupportedVersion,
    InvalidPath,
}

impl AppMarkerError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "unsupported app version",
            Self::InvalidPath => "invalid app path",
        }
    }
}

/// `path == None` means the publisher omitted it (canonical `/`).
/// A present path, including `""`, is checked as written.
pub fn validate_app_marker(v: u8, path: Option<&str>) -> Result<AppMarker, AppMarkerError> {
    if v != 1 {
        return Err(AppMarkerError::UnsupportedVersion);
    }
    let path = path.unwrap_or("/");
    if !app_path_ok(path) {
        return Err(AppMarkerError::InvalidPath);
    }
    Ok(AppMarker {
        v: 1,
        path: path.to_string(),
    })
}

/// Marker present iff TXT `app` is present. Absent → `Ok(None)` and `path`
/// is left opaque. Present `app` must be byte-exact `v1`; `path`, if
/// present, uses [`validate_app_marker`].
pub fn validate_txt_app_marker(
    txt: &BTreeMap<String, String>,
) -> Result<Option<AppMarker>, AppMarkerError> {
    match txt.get("app").map(String::as_str) {
        None => Ok(None),
        Some("v1") => {
            let path = txt.get("path").map(String::as_str);
            validate_app_marker(1, path).map(Some)
        }
        Some(_) => Err(AppMarkerError::UnsupportedVersion),
    }
}

/// Accept or reject a publish TXT map. On success with a root path, `path`
/// is omitted so the directory does not list `path=/`. Other keys are kept.
/// Without `app`, the map is returned unchanged.
pub fn publish_txt_with_marker(
    mut txt: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, AppMarkerError> {
    match validate_txt_app_marker(&txt)? {
        None => Ok(txt),
        Some(marker) => {
            txt.insert("app".to_string(), "v1".to_string());
            if marker.path == "/" {
                txt.remove("path");
            } else {
                txt.insert("path".to_string(), marker.path);
            }
            Ok(txt)
        }
    }
}

/// Mirrors `contract.ts` `appPath` / `appPathAllowed`.
fn app_path_ok(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains("//")
        && !path.contains('\\')
        && !path
            .chars()
            .any(|c| ('\u{0000}'..='\u{001f}').contains(&c) || c == '\u{007f}')
        && !has_scheme(path)
}

/// `(?:^|/)[a-z][a-z\d+.-]*:` case-insensitive, ASCII only (no JS `u` flag).
fn has_scheme(path: &str) -> bool {
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let boundary = i == 0 || bytes[i - 1] == b'/';
        if boundary && bytes[i].is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < bytes.len() && is_scheme_byte(bytes[j]) {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b':' {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_scheme_byte(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte.is_ascii_digit() || matches!(byte, b'+' | b'.' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        name: String,
        #[serde(default)]
        marker: Option<MarkerIn>,
        #[serde(default)]
        txt: Option<BTreeMap<String, String>>,
        expect: Option<AppMarker>,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        opaque: bool,
    }

    #[derive(Deserialize)]
    struct MarkerIn {
        v: u8,
        #[serde(default)]
        path: Option<String>,
    }

    #[test]
    fn shared_app_marker_fixtures() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../hello-mesh-web/src/lib/miniapp/fixtures/app-marker");
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|ent| ent.unwrap().path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "app-marker fixtures missing");

        for path in files {
            let bytes = std::fs::read(&path).unwrap();
            let cases: Vec<Fixture> = serde_json::from_slice(&bytes)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(!cases.is_empty(), "{}", path.display());
            for fixture in cases {
                match (&fixture.marker, &fixture.txt) {
                    (Some(marker), None) => {
                        let result =
                            validate_app_marker(marker.v, marker.path.as_deref()).map(Some);
                        assert_marker(&fixture, result);
                    }
                    (None, Some(txt)) => {
                        let result = validate_txt_app_marker(txt);
                        if fixture.opaque {
                            assert_eq!(result, Ok(None), "{}", fixture.name);
                            assert!(fixture.error.is_none(), "{}", fixture.name);
                        } else {
                            assert_marker(&fixture, result);
                        }
                    }
                    _ => panic!("{}: set marker or txt, not both", fixture.name),
                }
            }
        }
    }

    fn assert_marker(fixture: &Fixture, result: Result<Option<AppMarker>, AppMarkerError>) {
        match &fixture.error {
            Some(error) => {
                let err = result.expect_err(&fixture.name);
                assert_eq!(err.as_str(), error, "{}", fixture.name);
                assert!(fixture.expect.is_none(), "{}", fixture.name);
            }
            None => {
                assert_eq!(
                    result.expect(&fixture.name).as_ref(),
                    fixture.expect.as_ref(),
                    "{}",
                    fixture.name
                );
            }
        }
    }

    #[test]
    fn publish_txt_omits_root_path_and_leaves_plain_txt() {
        let mut marked = BTreeMap::new();
        marked.insert("app".into(), "v1".into());
        marked.insert("path".into(), "/".into());
        marked.insert("proto".into(), "http".into());
        let stored = publish_txt_with_marker(marked).unwrap();
        assert_eq!(stored.get("app").map(String::as_str), Some("v1"));
        assert!(!stored.contains_key("path"));
        assert_eq!(stored.get("proto").map(String::as_str), Some("http"));

        let mut plain = BTreeMap::new();
        plain.insert("path".into(), "print".into());
        let stored = publish_txt_with_marker(plain.clone()).unwrap();
        assert_eq!(stored, plain);

        let mut bad = BTreeMap::new();
        bad.insert("app".into(), "v2".into());
        assert_eq!(
            publish_txt_with_marker(bad).unwrap_err(),
            AppMarkerError::UnsupportedVersion
        );
    }
}
