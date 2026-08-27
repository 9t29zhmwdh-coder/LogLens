//! Syslog listener.
//!
//! Unlike every other collector in this crate, this one does not read a
//! source: appliances push to it. That inverts two things. There is no
//! backlog to catch up on when the listener starts, and a slow consumer
//! costs datagrams rather than memory, which is why UDP handling stays free
//! of any per-line await beyond the channel send.

use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, watch};

use crate::clustering::ClusterGrouper;
use crate::models::log_entry::{LogSource, NormalizedEntry, SyslogTransport};
use crate::normalizer::syslog;
use crate::plugin::network::ClassifierRegistry;

/// The largest datagram we accept. RFC 5424 sets no ceiling, but 64 KiB is
/// the practical IPv4 datagram limit and well beyond what appliances emit.
const MAX_DATAGRAM: usize = 65_535;

pub async fn run(
    source: LogSource,
    bind: String,
    protocol: SyslogTransport,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    cancel: watch::Receiver<bool>,
) {
    let classifiers = Arc::new(ClassifierRegistry::with_builtins());
    let mut tasks = Vec::new();

    if protocol.accepts_udp() {
        tasks.push(tokio::spawn(run_udp(
            source.clone(),
            bind.clone(),
            tx.clone(),
            grouper.clone(),
            classifiers.clone(),
            cancel.clone(),
        )));
    }
    if protocol.accepts_tcp() {
        tasks.push(tokio::spawn(run_tcp(
            source,
            bind,
            tx,
            grouper,
            classifiers,
            cancel,
        )));
    }
    for task in tasks {
        let _ = task.await;
    }
}

async fn run_udp(
    source: LogSource,
    bind: String,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    classifiers: Arc<ClassifierRegistry>,
    mut cancel: watch::Receiver<bool>,
) {
    let socket = match UdpSocket::bind(&bind).await {
        Ok(socket) => socket,
        Err(e) => {
            tracing::error!("syslog: cannot bind UDP {bind}: {e}");
            return;
        }
    };
    tracing::info!("syslog: listening on UDP {bind}");

    let mut buffer = vec![0u8; MAX_DATAGRAM];
    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() { break; }
            }
            received = socket.recv_from(&mut buffer) => {
                let Ok((len, _peer)) = received else { continue };
                // One datagram may carry several lines.
                let text = String::from_utf8_lossy(&buffer[..len]);
                for line in text.lines() {
                    emit(line, &source, &grouper, &classifiers, &tx).await;
                }
            }
        }
    }
}

async fn run_tcp(
    source: LogSource,
    bind: String,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    classifiers: Arc<ClassifierRegistry>,
    mut cancel: watch::Receiver<bool>,
) {
    let listener = match TcpListener::bind(&bind).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("syslog: cannot bind TCP {bind}: {e}");
            return;
        }
    };
    tracing::info!("syslog: listening on TCP {bind}");

    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() { break; }
            }
            accepted = listener.accept() => {
                let Ok((stream, _peer)) = accepted else { continue };
                tokio::spawn(serve_stream(
                    stream,
                    source.clone(),
                    tx.clone(),
                    grouper.clone(),
                    classifiers.clone(),
                    cancel.clone(),
                ));
            }
        }
    }
}

/// Reads one TCP connection. Non-transparent framing (one line per message)
/// is assumed, which is what appliances send; octet-counted framing per
/// RFC 6587 is not yet handled.
async fn serve_stream(
    stream: TcpStream,
    source: LogSource,
    tx: mpsc::Sender<NormalizedEntry>,
    grouper: Arc<ClusterGrouper>,
    classifiers: Arc<ClassifierRegistry>,
    mut cancel: watch::Receiver<bool>,
) {
    let mut lines = BufReader::new(stream).lines();
    loop {
        tokio::select! {
            _ = cancel.changed() => {
                if *cancel.borrow() { break; }
            }
            next = lines.next_line() => {
                match next {
                    Ok(Some(line)) => emit(&line, &source, &grouper, &classifiers, &tx).await,
                    _ => break,
                }
            }
        }
    }
}

/// Normalizes and forwards one line. A line that is not syslog is dropped
/// rather than guessed at: on a listener socket, malformed input is either a
/// port scan or a misconfigured sender, and neither belongs in the log view.
async fn emit(
    line: &str,
    source: &LogSource,
    grouper: &ClusterGrouper,
    classifiers: &ClassifierRegistry,
    tx: &mpsc::Sender<NormalizedEntry>,
) {
    if line.trim().is_empty() {
        return;
    }
    let Some(mut entry) = syslog::normalize_line(line, source, classifiers) else {
        tracing::debug!("syslog: dropped unparseable line");
        return;
    };
    grouper.process(&mut entry);
    let _ = tx.send(entry).await;
}
