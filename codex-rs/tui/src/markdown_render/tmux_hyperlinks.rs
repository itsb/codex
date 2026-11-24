//! Best-effort tmux capability discovery for the local TUI, never for telemetry detection.
//!
//! Query the active server rather than installed versions or configured feature patterns. Every
//! attached client must support links; nested multiplexers and unavailable probes keep visible URLs.
//! This is a startup snapshot, like the other web-link presentation heuristics.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_OUTPUT: usize = 8192;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TmuxHyperlinks {
    Supported,
    Unknown,
}

pub(super) async fn probe() -> TmuxHyperlinks {
    let Some(tmux) = std::env::var_os("TMUX") else {
        return TmuxHyperlinks::Unknown;
    };
    let Some(tmux) = tmux.to_str() else {
        return TmuxHyperlinks::Unknown;
    };
    // Socket paths may contain commas. The two final components are server PID and session ID.
    let mut parts = tmux.rsplitn(3, ',');
    let _session = parts.next();
    let _pid = parts.next();
    let Some(socket) = parts
        .next()
        .filter(|socket| Path::new(socket).is_absolute())
    else {
        return TmuxHyperlinks::Unknown;
    };

    // Do not execute a repository-provided helper through PATH. Nonstandard installations fall
    // back to visible destinations. Keep this separate from environment-only terminal detection.
    let Some(executable) = [
        "/opt/homebrew/bin/tmux",
        "/usr/local/bin/tmux",
        "/usr/bin/tmux",
    ]
    .into_iter()
    .find(|path| Path::new(path).is_file()) else {
        return TmuxHyperlinks::Unknown;
    };
    let mut command = Command::new(executable);
    command.current_dir("/").args([
        "-N", // Never start a server as a side effect of detection.
        "-S",
        socket,
        "list-clients",
        "-F",
        "#{client_termname}\t#{client_termfeatures}",
    ]);
    query(&mut command).await
}

async fn query(command: &mut Command) -> TmuxHyperlinks {
    let result = tokio::time::timeout(PROBE_TIMEOUT, async {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .ok()?;
        let mut output = Vec::new();
        child
            .stdout
            .take()?
            .take((MAX_OUTPUT + 1) as u64)
            .read_to_end(&mut output)
            .await
            .ok()?;
        if output.len() > MAX_OUTPUT || !child.wait().await.ok()?.success() {
            return None;
        }
        Some(parse_clients(std::str::from_utf8(&output).ok()?))
    })
    .await;
    result.ok().flatten().unwrap_or(TmuxHyperlinks::Unknown)
}

fn parse_clients(output: &str) -> TmuxHyperlinks {
    let mut clients = output.lines().peekable();
    if clients.peek().is_none() {
        return TmuxHyperlinks::Unknown;
    }
    for client in clients {
        let Some((term, features)) = client.split_once('\t') else {
            return TmuxHyperlinks::Unknown;
        };
        if term.is_empty()
            || term == "dumb"
            || term.starts_with("tmux")
            || term.starts_with("screen")
            || !features.split(',').any(|feature| feature == "hyperlinks")
        {
            return TmuxHyperlinks::Unknown;
        }
    }
    TmuxHyperlinks::Supported
}

#[cfg(test)]
#[path = "tmux_hyperlinks_tests.rs"]
mod tests;
