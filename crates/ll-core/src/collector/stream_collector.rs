use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::models::log_entry::{NormalizedEntry, LogSource};
use crate::normalizer::stacktrace_detector::StacktraceAccumulator;
use crate::clustering::ClusterGrouper;
use super::{emit_line, flush_accumulator};

pub async fn run_stdin(
    source: LogSource,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut accumulator = StacktraceAccumulator::new();

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() { break; }
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
