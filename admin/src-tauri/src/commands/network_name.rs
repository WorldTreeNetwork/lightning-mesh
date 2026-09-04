// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Fleet network name (client AP SSID). Stages a wireless env and runs
//! `deploy/openwrt/update-fleet.sh --wireless` — the same mjolnir-apply path
//! as the CLI. This is a radio network name, not a guild.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Per-node outcome of a fleet `--wireless` apply.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyReport {
    pub updated: Vec<String>,
    pub skipped: Vec<String>,
    pub halted: Option<String>,
    pub ok: bool,
    pub log: String,
}

/// SSID is 32 octets. Reject empty (after trim), oversize, and NUL/CR/LF.
fn ssid_ok(name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("network name is empty".into());
    }
    if name.len() > 32 {
        return Err("network name exceeds the 32-octet SSID limit".into());
    }
    if name.bytes().any(|b| b == 0 || b == b'\n' || b == b'\r') {
        return Err("network name contains a forbidden control character".into());
    }
    Ok(())
}

fn shell_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Shape mjolnir-apply can `set -a; . file`. Open encryption stays representable.
fn render_wireless_env(ssid: &str) -> String {
    let quoted = shell_single_quote(ssid);
    format!(
        "# Network name phones join — not a guild. Association is not membership.\n\
         # Staged by Lightning Admin; sourced by mjolnir-apply (set -a).\n\
         CLIENT_SSID={quoted}\n\
         # OPEN client AP, no password. Do not mix open + PSK across nodes.\n\
         CLIENT_ENC='none'\n\
         CLIENT_KEY=''\n\
         CLIENT_AP_2G_ENC='none'\n"
    )
}

fn parse_node_header(line: &str) -> Option<String> {
    let t = line.trim();
    if !t.starts_with("=====") {
        return None;
    }
    let inner = t.trim_matches('=').trim();
    if inner.is_empty() || inner.starts_with("fleet rollout") {
        return None;
    }
    let name = inner
        .split(|c: char| c.is_whitespace() || c == '(')
        .find(|s| !s.is_empty())?;
    Some(name.to_string())
}

fn halt_name(line: &str) -> Option<String> {
    let after = line.split("ROLLOUT HALTED at ").nth(1)?;
    let name = after.split_whitespace().next()?;
    let name = name.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn parse_name_list(rest: &str) -> Vec<String> {
    let rest = rest.trim();
    if rest.is_empty() || rest == "none" {
        return Vec::new();
    }
    rest.split_whitespace().map(str::to_string).collect()
}

/// Parse `update-fleet.sh` stdout: node headers, skip, halt, OK, summary.
fn parse_fleet_report(stdout: &str) -> ApplyReport {
    let mut updated = Vec::new();
    let mut skipped = Vec::new();
    let mut halted = None;
    let mut current: Option<String> = None;
    let mut summary_updated: Option<Vec<String>> = None;
    let mut summary_skipped: Option<Vec<String>> = None;

    for line in stdout.lines() {
        if let Some(name) = parse_node_header(line) {
            current = Some(name);
            continue;
        }
        if line.contains("UNREACHABLE") && line.contains("skipping") {
            if let Some(name) = current.take() {
                if !skipped.contains(&name) {
                    skipped.push(name);
                }
            }
            continue;
        }
        if line.contains("ROLLOUT HALTED") {
            if halted.is_none() {
                halted = halt_name(line).or_else(|| current.take());
            }
            continue;
        }
        if let Some(name) = current.as_ref() {
            let marker = format!("{name}: OK");
            if line.contains(&marker) {
                if !updated.contains(name) {
                    updated.push(name.clone());
                }
                current = None;
                continue;
            }
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("updated:") {
            summary_updated = Some(parse_name_list(rest));
        } else if let Some(rest) = trimmed.strip_prefix("unreachable:") {
            summary_skipped = Some(parse_name_list(rest));
        }
    }

    if updated.is_empty() {
        if let Some(names) = summary_updated {
            updated = names;
        }
    }
    if skipped.is_empty() {
        if let Some(names) = summary_skipped {
            skipped = names;
        }
    }

    ApplyReport {
        ok: halted.is_none(),
        updated,
        skipped,
        halted,
        log: stdout.to_string(),
    }
}

fn update_fleet_under(root: &Path) -> PathBuf {
    root.join("deploy/openwrt/update-fleet.sh")
}

fn missing_script_error() -> String {
    "could not find deploy/openwrt/update-fleet.sh (set LIGHTNING_MESH_ROOT to the lightning-mesh checkout; Apply needs overlay SSH and fleet-nodes.conf)".into()
}

fn find_update_fleet() -> Result<PathBuf, String> {
    if let Ok(root) = std::env::var("LIGHTNING_MESH_ROOT") {
        let p = update_fleet_under(Path::new(&root));
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "LIGHTNING_MESH_ROOT={root} has no deploy/openwrt/update-fleet.sh"
        ));
    }

    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            starts.push(parent.to_path_buf());
        }
    }
    for start in starts {
        for dir in start.ancestors() {
            let p = update_fleet_under(dir);
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err(missing_script_error())
}

struct TempEnv(PathBuf);

impl Drop for TempEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn write_temp_env(body: &str) -> Result<TempEnv, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "lightning-admin-wireless-{}-{nanos}.env",
        std::process::id()
    ));
    std::fs::write(&path, body).map_err(|e| format!("could not write wireless env: {e}"))?;
    Ok(TempEnv(path))
}

fn spawn_error(err: std::io::Error) -> String {
    #[cfg(windows)]
    {
        format!(
            "failed to run update-fleet.sh ({err}). Apply needs bash and deploy/openwrt/update-fleet.sh (set LIGHTNING_MESH_ROOT)."
        )
    }
    #[cfg(not(windows))]
    {
        format!("failed to run update-fleet.sh: {err}")
    }
}

/// Stage `CLIENT_SSID` and run the existing sequential health-gated fleet apply.
/// No live UCI SSH mutation. Takes as long as the script; no 30s kill.
#[tauri::command]
pub async fn apply_network_name(name: String) -> Result<ApplyReport, String> {
    ssid_ok(&name)?;
    let ssid = name.trim();
    let script = find_update_fleet()?;
    if !script.is_file() {
        return Err(missing_script_error());
    }

    let tmp = write_temp_env(&render_wireless_env(ssid))?;

    let mut cmd = tokio::process::Command::new("bash");
    cmd.arg(&script)
        .arg("--wireless")
        .arg(&tmp.0)
        .kill_on_drop(true);

    let output = cmd.output().await.map_err(spawn_error)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut log = stdout.into_owned();
    if !stderr.is_empty() {
        if !log.is_empty() && !log.ends_with('\n') {
            log.push('\n');
        }
        log.push_str(&stderr);
    }

    let mut report = parse_fleet_report(&log);
    report.log = log;
    report.ok = output.status.success() && report.halted.is_none();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_values(body: &str) -> String {
        body.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn lightning_bolt_is_valid_three_octet_ssid() {
        assert_eq!("⚡".len(), 3);
        assert!(ssid_ok("⚡").is_ok());
    }

    #[test]
    fn empty_rejected() {
        assert!(ssid_ok("").is_err());
        assert!(ssid_ok("   ").is_err());
        assert!(ssid_ok("\n").is_err());
    }

    #[test]
    fn octet_limit_32_accepted_33_rejected() {
        let s32 = "a".repeat(32);
        let s33 = "a".repeat(33);
        assert!(ssid_ok(&s32).is_ok());
        assert!(ssid_ok(&s33).is_err());
    }

    #[test]
    fn newline_and_nul_rejected() {
        assert!(ssid_ok("foo\nbar").is_err());
        assert!(ssid_ok("foo\rbar").is_err());
        assert!(ssid_ok("foo\0bar").is_err());
    }

    #[test]
    fn render_open_network_name_not_guild_value() {
        let body = render_wireless_env("Lightning Mesh");
        assert!(body.contains("CLIENT_SSID='Lightning Mesh'"));
        assert!(body.contains("CLIENT_ENC='none'"));
        assert!(body.contains("CLIENT_KEY=''"));
        assert!(body.contains("CLIENT_AP_2G_ENC='none'"));
        let values = env_values(&body);
        assert!(
            !values.to_lowercase().contains("guild"),
            "values must not contain guild: {values}"
        );
        assert!(
            body.to_lowercase().contains("not a guild"),
            "comment may mention not a guild"
        );
    }

    #[test]
    fn render_escapes_single_quote_for_shell() {
        let body = render_wireless_env("foo'bar");
        assert!(
            body.contains("CLIENT_SSID='foo'\\''bar'"),
            "expected POSIX single-quote escape, got:\n{body}"
        );
    }

    #[test]
    fn parse_skip_and_halt_lines() {
        let stdout = "\
===== ap3000 (GL-AP3000) — root@10.254.1.2 =====
>> UNREACHABLE — skipping (garage, powered off)

===== m3000 (GL-MT3000) — root@10.254.3.4 =====
>> m3000: OK (babel routes to 2 neighbour(s))

===== tr3000 (GL-MT3000) — root@10.254.5.6 =====
>> ROLLOUT HALTED at tr3000 — result='FAILED'. If unreadable,
>> Already updated this run: m3000. Re-running is safe (idempotent).
";
        let report = parse_fleet_report(stdout);
        assert_eq!(report.updated, vec!["m3000"]);
        assert_eq!(report.skipped, vec!["ap3000"]);
        assert_eq!(report.halted.as_deref(), Some("tr3000"));
        assert!(!report.ok);
    }

    #[test]
    fn parse_summary_when_stream_ok_missing() {
        let stdout = "\
===== fleet rollout summary =====
updated:     leaf1 leaf2
unreachable: none
";
        let report = parse_fleet_report(stdout);
        assert_eq!(report.updated, vec!["leaf1", "leaf2"]);
        assert!(report.skipped.is_empty());
        assert!(report.halted.is_none());
        assert!(report.ok);
    }
}
