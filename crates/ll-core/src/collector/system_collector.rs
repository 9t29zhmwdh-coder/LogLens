use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::models::log_entry::{NormalizedEntry, LogSource};
use crate::normalizer::stacktrace_detector::StacktraceAccumulator;
use crate::clustering::ClusterGrouper;
use super::{emit_line, flush_accumulator};

/// macOS Unified Logging via `log stream`
pub async fn run_macos(
    source: LogSource,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    #[cfg(not(target_os = "macos"))]
    {
        tracing::warn!("macOS Unified Logging only available on macOS");
        return;
    }

    let mut child = match Command::new("log")
        .args(["stream", "--style", "json", "--level", "debug"])
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to spawn `log stream`: {}", e);
            return;
        }
    };

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();
    let mut accumulator = StacktraceAccumulator::new();

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() {
                    let _ = child.kill().await;
                    break;
                }
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) if !l.trim().is_empty() => {
                        emit_line(&l, &source, &mut accumulator, &grouper, &tx).await;
                    }
                    Ok(None) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }

    flush_accumulator(&mut accumulator, &source, &grouper, &tx).await;
}

/// Linux systemd journal via `journalctl -f`
pub async fn run_journald(
    source: LogSource,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    let mut child = match Command::new("journalctl")
        .args(["-f", "-o", "json"])
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to spawn `journalctl`: {}", e);
            return;
        }
    };

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();
    let mut accumulator = StacktraceAccumulator::new();

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() {
                    let _ = child.kill().await;
                    break;
                }
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) if !l.trim().is_empty() => {
                        emit_line(&l, &source, &mut accumulator, &grouper, &tx).await;
                    }
                    Ok(None) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }

    flush_accumulator(&mut accumulator, &source, &grouper, &tx).await;
}

/// Windows Event Log via PowerShell `Get-WinEvent`
pub async fn run_windows_event(
    source: LogSource,
    channel: String,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    #[cfg(not(target_os = "windows"))]
    {
        tracing::warn!("Windows Event Log only available on Windows");
        return;
    }

    let script = format!(
        "Get-WinEvent -LogName '{}' -MaxEvents 50 | ConvertTo-Json",
        channel
    );

    let mut child = match Command::new("powershell")
        .args(["-NonInteractive", "-Command", &script])
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to spawn powershell: {}", e);
            return;
        }
    };

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();
    let mut accumulator = StacktraceAccumulator::new();

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() {
                    let _ = child.kill().await;
                    break;
                }
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) if !l.trim().is_empty() => {
                        emit_line(&l, &source, &mut accumulator, &grouper, &tx).await;
                    }
                    Ok(None) | Err(_) => break,
                    _ => {}
                }
            }
        }
    }

    flush_accumulator(&mut accumulator, &source, &grouper, &tx).await;
}
