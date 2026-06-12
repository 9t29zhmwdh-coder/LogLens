use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use bollard::Docker;
use bollard::container::LogsOptions;
use futures_util::StreamExt;
use crate::models::log_entry::{NormalizedEntry, LogSource};
use crate::normalizer::stacktrace_detector::StacktraceAccumulator;
use crate::clustering::ClusterGrouper;
use super::{emit_line, flush_accumulator};

pub async fn run_container(
    source: LogSource,
    container_id: String,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    mut cancel: watch::Receiver<bool>,
) {
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Docker connect failed: {}", e);
            return;
        }
    };

    let opts = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: "50".to_string(),
        ..Default::default()
    };

    let mut stream = docker.logs(&container_id, Some(opts));
    let mut accumulator = StacktraceAccumulator::new();

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() { break; }
            }
            msg = stream.next() => {
                match msg {
                    Some(Ok(output)) => {
                        let line = match output {
                            bollard::container::LogOutput::StdOut { message } |
                            bollard::container::LogOutput::StdErr { message } => {
                                String::from_utf8_lossy(&message).trim_end().to_string()
                            }
                            _ => continue,
                        };
                        if !line.is_empty() {
                            emit_line(&line, &source, &mut accumulator, &grouper, &tx).await;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::warn!("Docker log stream error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    flush_accumulator(&mut accumulator, &source, &grouper, &tx).await;
}

pub async fn run_service(
    source: LogSource,
    service_name: String,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    cancel: watch::Receiver<bool>,
) {
    let docker = match Docker::connect_with_local_defaults() {
        Ok(d) => d,
        Err(e) => {
            tracing::error!("Docker connect failed: {}", e);
            return;
        }
    };

    // Resolve all container IDs matching the service label
    let mut filters = std::collections::HashMap::new();
    filters.insert("label", vec![format!("com.docker.compose.service={}", service_name)]);

    let containers = match docker.list_containers(Some(bollard::container::ListContainersOptions {
        filters,
        ..Default::default()
    })).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("list_containers failed: {}", e);
            return;
        }
    };

    let mut handles = Vec::new();
    for container in containers {
        if let Some(id) = container.id {
            let src = source.clone();
            let t = tx.clone();
            let g = grouper.clone();
            let c = cancel.clone();
            handles.push(tokio::spawn(async move {
                run_container(src, id, t, g, c).await;
            }));
        }
    }

    for h in handles {
        let _ = h.await;
    }
}
