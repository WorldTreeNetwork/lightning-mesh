// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Stock OpenWrt bring-up. Runs `deploy/openwrt/provision/provision-node.sh`.
//! That script installs, enrolls the fleet line, and prints the lamp.
//! Health is the same script with `--check` and does not change the router.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvisionReport {
    pub ok: bool,
    pub log: String,
    pub node_id: String,
    pub lamp: String,
}

fn shell_token_ok(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// `root@192.168.1.1` or `root@hostname`. No spaces, no second `@`.
fn host_ok(host: &str) -> bool {
    let Some((user, rest)) = host.split_once('@') else {
        return false;
    };
    if user.is_empty() || rest.is_empty() || rest.contains('@') {
        return false;
    }
    shell_token_ok(user)
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' || c == ':')
}

fn script_under(dir: &Path) -> PathBuf {
    dir.join("deploy/openwrt/provision/provision-node.sh")
}

fn find_script() -> Result<PathBuf, String> {
    if let Ok(root) = std::env::var("LIGHTNING_MESH_ROOT") {
        let p = script_under(Path::new(&root));
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "LIGHTNING_MESH_ROOT={root} has no deploy/openwrt/provision/provision-node.sh"
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
            let p = script_under(dir);
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err("deploy/openwrt/provision/provision-node.sh not found (set LIGHTNING_MESH_ROOT)".into())
}

fn field<'a>(log: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}: ");
    log.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .unwrap_or("")
}

fn report_from(ok: bool, log: String) -> ProvisionReport {
    ProvisionReport {
        node_id: field(&log, "node_id").to_string(),
        lamp: field(&log, "lamp").to_string(),
        ok,
        log,
    }
}

async fn run(script: &Path, args: &[&str]) -> Result<ProvisionReport, String> {
    let output = tokio::process::Command::new("bash")
        .arg(script)
        .args(args)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|err| format!("failed to run provision-node.sh: {err}"))?;
    let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        if !log.is_empty() && !log.ends_with('\n') {
            log.push('\n');
        }
        log.push_str(&stderr);
    }
    Ok(report_from(output.status.success(), log))
}

/// Install Lightning Mesh on a stock OpenWrt router, append its fleet line,
/// and render peers. `host` is `root@<addr>`.
#[tauri::command]
pub async fn provision_stock_node(
    host: String,
    name: String,
    group: String,
) -> Result<ProvisionReport, String> {
    if !host_ok(&host) {
        return Err("host must look like root@192.168.1.1".into());
    }
    if !shell_token_ok(&name) || !shell_token_ok(&group) {
        return Err("name and group must be letters, numbers, '.', '_' or '-'".into());
    }
    let script = find_script()?;
    run(&script, &["--name", &name, "--group", &group, &host]).await
}

/// Read-only lamp, radio, and node id. Does not install or edit the inventory.
#[tauri::command]
pub async fn provision_node_health(host: String) -> Result<ProvisionReport, String> {
    if !host_ok(&host) {
        return Err("host must look like root@192.168.1.1".into());
    }
    let script = find_script()?;
    run(&script, &["--check", &host]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_accepts_root_at_ipv4() {
        assert!(host_ok("root@192.168.1.1"));
        assert!(host_ok("root@tr3000-b.lan"));
        assert!(!host_ok("192.168.1.1"));
        assert!(!host_ok("root@192.168.1.1;id"));
        assert!(!host_ok("root@10.0.0.1 extra"));
    }

    #[test]
    fn report_reads_node_and_lamp() {
        let report = report_from(
            true,
            "node_id: abc\nlamp: flashing-red\nlamp_reason: mesh\n".into(),
        );
        assert!(report.ok);
        assert_eq!(report.node_id, "abc");
        assert_eq!(report.lamp, "flashing-red");
    }
}
