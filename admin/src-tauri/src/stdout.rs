// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Finite, allowlisted host transcripts. The frontend supplies only a source name.

use std::process::Child;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;

use crate::canvas::CanvasState;

const TIMEOUT: Duration = Duration::from_secs(5);
const CAP: usize = 65536;
const CANCELLED: &str = "canvas fill superseded";

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FillStatus {
    Success,
    Failure,
    Truncated,
    // This wire status is used only when running on another host platform.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    Unsupported,
    Refused,
}

#[derive(Debug, Serialize)]
pub struct FillResult {
    pub source: String,
    pub status: FillStatus,
    pub exit_code: Option<i32>,
    pub text: String,
    pub truncated: bool,
}

impl FillResult {
    fn message(source: &str, status: FillStatus, text: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            status,
            exit_code: None,
            text: text.into(),
            truncated: false,
        }
    }
}

struct Source {
    path: &'static str,
    args: &'static [&'static str],
}

fn lookup(name: &str) -> Option<Source> {
    match name {
        "ifconfig" => Some(Source {
            path: "/sbin/ifconfig",
            args: &[],
        }),
        "netstat" => Some(Source {
            path: "/usr/sbin/netstat",
            args: &["-ib"],
        }),
        _ => None,
    }
}

#[derive(Default)]
struct Running {
    open: bool,
    generation: u64,
    child: Option<Child>,
}

impl Running {
    fn reap(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        let Some(mut child) = self.child.take() else {
            return Ok(None);
        };
        // kill may fail if the process has already exited; wait still reaps it.
        let _ = child.kill();
        loop {
            match child.wait() {
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                result => return result.map(Some),
            }
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.reap();
    }
}

/// Owned by CanvasState: cancellation and spawn share one lock, so a new
/// generation cannot overlap the old child, even while its capture is pending.
#[derive(Default)]
pub(crate) struct SourceState {
    running: Mutex<Running>,
}

impl SourceState {
    pub(crate) fn activate(&self) {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        running.open = true;
    }

    pub(crate) fn deactivate(&self) {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        running.open = false;
        running.generation += 1;
        let _ = running.reap();
    }

    fn begin(&self) -> Result<u64, String> {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        running.generation += 1;
        running
            .reap()
            .map_err(|e| format!("Cannot reap source: {e}"))?;
        if !running.open {
            return Err(CANCELLED.into());
        }
        Ok(running.generation)
    }

    fn current(&self, generation: u64) -> Result<MutexGuard<'_, Running>, String> {
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if !running.open || running.generation != generation {
            return Err(CANCELLED.into());
        }
        Ok(running)
    }

    async fn fill(&self, name: &str) -> Result<FillResult, String> {
        let generation = self.begin()?;
        let Some(source) = lookup(name) else {
            return Ok(FillResult::message(
                name,
                FillStatus::Refused,
                "Unknown source refused.",
            ));
        };
        #[cfg(target_os = "macos")]
        {
            let mut command = std::process::Command::new(source.path);
            command.args(source.args);
            self.capture(generation, name, command, TIMEOUT).await
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (source.path, source.args, generation, TIMEOUT, CAP);
            Ok(FillResult::message(
                name,
                FillStatus::Unsupported,
                format!("{name} is supported only on macOS."),
            ))
        }
    }

    #[cfg(unix)]
    async fn capture(
        &self,
        generation: u64,
        name: &str,
        mut command: std::process::Command,
        timeout: Duration,
    ) -> Result<FillResult, String> {
        use std::io::Read;
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        use std::process::Stdio;
        use std::time::Instant;

        let started = Instant::now();
        let setup = || -> std::io::Result<UnixStream> {
            let mut running = self.current(generation).map_err(std::io::Error::other)?;
            let (reader, writer) = UnixStream::pair()?;
            reader.set_nonblocking(true)?;
            // One stream for both descriptors preserves their write order and
            // applies one shared cap; no reader threads or unbounded buffers.
            command
                .stdin(Stdio::null())
                .stdout(Stdio::from(OwnedFd::from(writer.try_clone()?)))
                .stderr(Stdio::from(OwnedFd::from(writer)));
            running.child = Some(command.spawn()?);
            Ok(reader)
        };
        let mut setup = setup;
        let reader = setup();
        // Command retains its stdio handles: close the parent's writer copies.
        drop(command);
        let mut reader = match reader {
            Ok(reader) => reader,
            Err(e) => {
                drop(self.current(generation)?);
                return Ok(FillResult::message(
                    name,
                    FillStatus::Failure,
                    format!("Cannot start source: {e}"),
                ));
            }
        };

        let mut bytes = Vec::with_capacity(CAP);
        let mut eof = false;
        let mut failure = None;
        let mut truncated = false;
        let mut failed_exit = false;
        let exit;
        loop {
            {
                let mut running = self.current(generation)?;
                if started.elapsed() >= timeout {
                    failure = Some(format!(
                        "Source timed out after {} seconds.",
                        timeout.as_secs_f64()
                    ));
                } else {
                    let mut buffer = [0_u8; 4096];
                    while bytes.len() < CAP && !eof {
                        let limit = buffer.len().min(CAP - bytes.len());
                        match reader.read(&mut buffer[..limit]) {
                            Ok(0) => eof = true,
                            Ok(n) => bytes.extend_from_slice(&buffer[..n]),
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                            Err(e) => {
                                failure = Some(format!("Cannot read source: {e}"));
                                break;
                            }
                        }
                    }
                    truncated = bytes.len() == CAP;
                }
                let status = running
                    .child
                    .as_mut()
                    .expect("current capture has a child")
                    .try_wait();
                match status {
                    Err(e) => failure = Some(format!("Cannot wait for source: {e}")),
                    Ok(Some(status)) if eof || truncated => {
                        failed_exit = !status.success();
                        exit = Some(status);
                        running.child.take(); // try_wait already reaped it.
                        break;
                    }
                    _ => {}
                }
                if failure.is_some() || truncated {
                    exit = running
                        .reap()
                        .map_err(|e| format!("Cannot reap source: {e}"))?;
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        drop(reader); // stop reading at the cap; the child has been reaped.
        drop(self.current(generation)?); // no cancelled generation can return a transcript.
        let exit_code = exit.and_then(|s| s.code());
        let status = if failure.is_some() || failed_exit {
            FillStatus::Failure
        } else if truncated {
            FillStatus::Truncated
        } else if exit.is_some_and(|s| s.success()) {
            FillStatus::Success
        } else {
            FillStatus::Failure
        };
        let mut text = String::from_utf8_lossy(&bytes).into_owned();
        if let Some(message) = failure {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&message);
        }
        Ok(FillResult {
            source: name.into(),
            status,
            exit_code,
            text,
            truncated,
        })
    }
}

/// A superseded fill rejects IPC rather than returning stale output for paint.
#[tauri::command]
pub async fn fill_canvas_source(
    state: tauri::State<'_, CanvasState>,
    name: String,
) -> Result<FillResult, String> {
    state.source.fill(&name).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> SourceState {
        let state = SourceState::default();
        state.activate();
        state
    }

    #[tokio::test]
    async fn unknown_source_is_refused_without_spawn() {
        let state = state();
        for name in [
            "unknown",
            "/sbin/ifconfig",
            "ifconfig; echo injected",
            "/usr/sbin/netstat",
            "netstat -ib",
            "netstat; echo injected",
        ] {
            let result = state.fill(name).await.unwrap();
            assert_eq!(result.status, FillStatus::Refused);
            assert_eq!(result.source, name);
            assert!(state.running.lock().unwrap().child.is_none());
        }
    }

    #[test]
    fn allowlisted_argv_is_fixed() {
        let source = lookup("ifconfig").unwrap();
        assert_eq!(source.path, "/sbin/ifconfig");
        assert!(source.args.is_empty());
        let source = lookup("netstat").unwrap();
        assert_eq!(source.path, "/usr/sbin/netstat");
        assert_eq!(source.args, &["-ib"]);
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn sources_are_unsupported_without_spawn() {
        let state = state();
        for name in ["ifconfig", "netstat"] {
            let result = state.fill(name).await.unwrap();
            assert_eq!(result.status, FillStatus::Unsupported);
            assert_eq!(result.source, name);
            assert!(result.text.contains(name));
            assert!(state.running.lock().unwrap().child.is_none());
        }
    }

    // Reuse this test binary as a tiny fixture, with no shell or external tools.
    #[cfg(unix)]
    fn fixture(mode: &str) -> std::process::Command {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "stdout::tests::fixture_child", "--nocapture"])
            .env("LIGHTNING_STDOUT_TEST_FIXTURE", mode);
        command
    }

    #[cfg(unix)]
    #[test]
    fn fixture_child() {
        use std::io::Write;
        let Ok(mode) = std::env::var("LIGHTNING_STDOUT_TEST_FIXTURE") else {
            return;
        };
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(30)),
            "cap" => {
                let _ = std::io::stdout().write_all(&vec![b'x'; CAP * 2]);
            }
            "output" => {
                std::io::stdout()
                    .write_all(b"<script>raw host bytes</script>\n")
                    .unwrap();
                std::io::stdout().flush().unwrap();
                std::io::stderr()
                    .write_all(b"stderr follows\xff\n")
                    .unwrap();
            }
            "failure" => std::process::exit(7),
            "stdin" => {
                use std::io::Read;
                assert_eq!(std::io::stdin().read(&mut [0; 1]).unwrap(), 0);
            }
            _ => panic!("unknown fixture"),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_kills_and_reaps_fixture() {
        let state = state();
        let result = state
            .capture(
                state.begin().unwrap(),
                "fixture",
                fixture("sleep"),
                Duration::from_millis(100),
            )
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Failure);
        assert!(result.text.contains("timed out"));
        assert!(!result.truncated);
        assert!(state.running.lock().unwrap().child.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cap_stops_capture_and_reaps_writer() {
        let state = state();
        let result = state
            .capture(state.begin().unwrap(), "fixture", fixture("cap"), TIMEOUT)
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Truncated);
        assert!(result.truncated);
        assert_eq!(result.text.len(), CAP);
        assert!(state.running.lock().unwrap().child.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn combines_raw_lossy_output_and_reports_nonzero_exit() {
        let state = state();
        let result = state
            .capture(
                state.begin().unwrap(),
                "fixture",
                fixture("output"),
                TIMEOUT,
            )
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result
            .text
            .contains("<script>raw host bytes</script>\nstderr follows\u{fffd}\n"));
        let result = state
            .capture(
                state.begin().unwrap(),
                "fixture",
                fixture("failure"),
                TIMEOUT,
            )
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Failure);
        assert_eq!(result.exit_code, Some(7));
        let result = state
            .capture(state.begin().unwrap(), "fixture", fixture("stdin"), TIMEOUT)
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Success);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn missing_binary_is_visible_failure() {
        let state = state();
        let result = state
            .capture(
                state.begin().unwrap(),
                "fixture",
                std::process::Command::new("/no-such-lightning-source"),
                TIMEOUT,
            )
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Failure);
        assert!(result.text.contains("Cannot start source"));
        assert!(state.running.lock().unwrap().child.is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn successful_empty_output_is_distinct_from_failure() {
        let state = state();
        let result = state
            .capture(
                state.begin().unwrap(),
                "fixture",
                std::process::Command::new("/usr/bin/true"),
                TIMEOUT,
            )
            .await
            .unwrap();
        assert_eq!(result.status, FillStatus::Success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.text.is_empty());
        assert!(!result.truncated);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn replacement_and_destroy_reap_and_drop_old_generations() {
        use std::sync::Arc;
        for destroy in [false, true] {
            let state = Arc::new(state());
            let generation = state.begin().unwrap();
            let owner = state.clone();
            let fill = tokio::spawn(async move {
                owner
                    .capture(generation, "ifconfig", fixture("sleep"), TIMEOUT)
                    .await
            });
            let pid = tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    if let Some(pid) = state.running.lock().unwrap().child.as_ref().map(Child::id) {
                        break pid;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            if destroy {
                state.deactivate();
            } else {
                let replacement = state.fill("netstat").await.unwrap();
                assert_eq!(replacement.source, "netstat");
                #[cfg(target_os = "macos")]
                assert_eq!(replacement.status, FillStatus::Success);
                #[cfg(not(target_os = "macos"))]
                assert_eq!(replacement.status, FillStatus::Unsupported);
            }
            assert_eq!(fill.await.unwrap().unwrap_err(), CANCELLED);
            assert!(state.running.lock().unwrap().child.is_none());
            // ECHILD proves cancellation waited, rather than merely signalling.
            assert_eq!(
                unsafe { libc::waitpid(pid as i32, std::ptr::null_mut(), libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
            assert!(state.current(generation).is_err());
            if destroy {
                assert!(state.begin().is_err());
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn real_sources_when_installed() {
        for name in ["ifconfig", "netstat"] {
            if !std::path::Path::new(lookup(name).unwrap().path).is_file() {
                continue;
            }
            let result = state().fill(name).await.unwrap();
            assert_eq!(result.source, name);
            assert_eq!(result.status, FillStatus::Success);
            assert_eq!(result.exit_code, Some(0));
            assert!(!result.text.is_empty());
        }
    }
}
