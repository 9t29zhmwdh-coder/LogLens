pub mod file_collector;
pub mod docker_collector;
pub mod system_collector;
pub mod stream_collector;

use std::sync::Arc;
use tokio::sync::mpsc;
use dashmap::DashMap;
use anyhow::Result;
use crate::models::log_entry::{NormalizedEntry, LogSource, LogSourceKind};
use crate::normalizer::stacktrace_detector::StacktraceAccumulator;
use crate::normalizer;
use crate::clustering::ClusterGrouper;

pub struct CollectorHandle {
    pub source: LogSource,
    cancel: tokio::sync::watch::Sender<bool>,
}

pub struct LogCollector {
    handles: DashMap<String, CollectorHandle>,
    pub tx: mpsc::Sender<NormalizedEntry>,
    pub grouper: Arc<ClusterGrouper>,
}

impl LogCollector {
    pub fn new(tx: mpsc::Sender<NormalizedEntry>) -> Self {
        Self {
            handles: DashMap::new(),
            tx,
            grouper: Arc::new(ClusterGrouper::new()),
        }
    }

    pub async fn watch(&self, source: LogSource) -> Result<()> {
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let tx = self.tx.clone();
        let grouper = self.grouper.clone();
        let src = source.clone();

        match &source.kind {
            LogSourceKind::File { path } => {
                let path = path.clone();
                tokio::spawn(file_collector::run(src, path, tx, grouper, cancel_rx));
            }
            LogSourceKind::Directory { path, pattern } => {
                let path = path.clone();
                let pattern = pattern.clone();
                tokio::spawn(file_collector::run_dir(src, path, pattern, tx, grouper, cancel_rx));
            }
            LogSourceKind::DockerContainer { container_id, .. } => {
                let cid = container_id.clone();
                tokio::spawn(docker_collector::run_container(src, cid, tx, grouper, cancel_rx));
            }
            LogSourceKind::DockerService { service_name } => {
                let svc = service_name.clone();
                tokio::spawn(docker_collector::run_service(src, svc, tx, grouper, cancel_rx));
            }
            LogSourceKind::Stdin => {
                tokio::spawn(stream_collector::run_stdin(src, tx, grouper, cancel_rx));
            }
            LogSourceKind::SystemMacos => {
                tokio::spawn(system_collector::run_macos(src, tx, grouper, cancel_rx));
            }
            LogSourceKind::Journald => {
                tokio::spawn(system_collector::run_journald(src, tx, grouper, cancel_rx));
            }
            LogSourceKind::WindowsEventLog { channel } => {
                let ch = channel.clone();
                tokio::spawn(system_collector::run_windows_event(src, ch, tx, grouper, cancel_rx));
            }
        }

        self.handles.insert(source.id.clone(), CollectorHandle { source, cancel: cancel_tx });
        Ok(())
    }

    pub fn stop(&self, source_id: &str) {
        if let Some((_, handle)) = self.handles.remove(source_id) {
            let _ = handle.cancel.send(true);
        }
    }

    pub fn list_sources(&self) -> Vec<LogSource> {
        self.handles.iter().map(|e| e.source.clone()).collect()
    }
}

/// Shared helper: feed a raw line through normalizer + clustering, emit via channel.
pub async fn emit_line(
    line: &str,
    source: &LogSource,
    accumulator: &mut StacktraceAccumulator,
    grouper: &ClusterGrouper,
    tx: &mpsc::Sender<NormalizedEntry>,
) {
    if let Some((msg, trace)) = accumulator.push(line) {
        if let Some(mut entry) = normalizer::normalize_line(&msg, source) {
            if !trace.is_empty() {
                entry.stacktrace = Some(trace);
            }
            grouper.process(&mut entry);
            let _ = tx.send(entry).await;
        }
    }
}

pub async fn flush_accumulator(
    accumulator: &mut StacktraceAccumulator,
    source: &LogSource,
    grouper: &ClusterGrouper,
    tx: &mpsc::Sender<NormalizedEntry>,
) {
    if let Some((msg, trace)) = accumulator.flush() {
        if let Some(mut entry) = normalizer::normalize_line(&msg, source) {
            if !trace.is_empty() {
                entry.stacktrace = Some(trace);
            }
            grouper.process(&mut entry);
            let _ = tx.send(entry).await;
        }
    }
}
