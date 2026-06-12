use std::sync::Arc;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use tokio::sync::{mpsc, watch};
use notify::{Watcher, RecursiveMode, RecommendedWatcher, EventKind};
use notify::event::ModifyKind;
use anyhow::Result;
use crate::models::log_entry::{NormalizedEntry, LogSource, LogSourceKind};
use crate::normalizer::stacktrace_detector::StacktraceAccumulator;
use crate::clustering::ClusterGrouper;
use super::{emit_line, flush_accumulator};

pub async fn run(
    source: LogSource,
    path: String,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    if let Err(e) = tail_file(&source, &path, &tx, &grouper, &mut cancel).await {
        tracing::error!("file_collector error for {}: {}", path, e);
    }
}

async fn tail_file(
    source: &LogSource,
    path: &str,
    tx: &mpsc::Sender<NormalizedEntry>,
    grouper: &ClusterGrouper,
    cancel: &mut watch::Receiver<bool>,
) -> Result<()> {
    let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(64);

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let _ = notify_tx.blocking_send(event);
            }
        },
        notify::Config::default(),
    )?;

    watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

    let mut file = File::open(path)?;
    // Seek to end for live-tail
    file.seek(SeekFrom::End(0))?;

    let mut accumulator = StacktraceAccumulator::new();

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() { break; }
            }
            Some(event) = notify_rx.recv() => {
                if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                    read_new_lines(&mut file, source, &mut accumulator, grouper, tx).await;
                }
            }
        }
    }

    flush_accumulator(&mut accumulator, source, grouper, tx).await;
    Ok(())
}

async fn read_new_lines(
    file: &mut File,
    source: &LogSource,
    accumulator: &mut StacktraceAccumulator,
    grouper: &ClusterGrouper,
    tx: &mpsc::Sender<NormalizedEntry>,
) {
    let mut reader = BufReader::new(&*file);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let l = line.trim_end_matches('\n').trim_end_matches('\r');
                if !l.is_empty() {
                    emit_line(l, source, accumulator, grouper, tx).await;
                }
            }
            Err(_) => break,
        }
    }
}

pub async fn run_dir(
    source: LogSource,
    path: String,
    pattern: Option<String>,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    let glob_pattern = pattern.unwrap_or_else(|| format!("{}/**/*.log", path));
    if let Ok(paths) = glob::glob(&glob_pattern) {
        let files: Vec<_> = paths.filter_map(|p| p.ok()).collect();
        let mut handles = Vec::new();

        for file_path in files {
            let src = LogSource {
                id: format!("{}/{}", source.id, file_path.display()),
                label: format!("{} ({})", source.label,
                    file_path.file_name().and_then(|n| n.to_str()).unwrap_or("?")),
                kind: LogSourceKind::File { path: file_path.to_string_lossy().to_string() },
                ..source.clone()
            };
            let (tx2, grouper2, cancel2) = (tx.clone(), grouper.clone(), cancel.clone());
            let p = file_path.to_string_lossy().to_string();
            handles.push(tokio::spawn(async move {
                run(src, p, tx2, grouper2, cancel2).await;
            }));
        }

        let _ = cancel.changed().await;
    }
}
