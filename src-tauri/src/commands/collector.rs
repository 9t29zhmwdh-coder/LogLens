use tauri::State;
use ll_core::models::log_entry::{LogSource, LogSourceKind, SyslogTransport};
use crate::state::AppState;
use crate::error::Result;

#[tauri::command]
pub async fn add_source(source: LogSource, state: State<'_, AppState>) -> Result<String> {
    let id = source.id.clone();
    ll_core::db::queries::insert_source(&state.pool, &source).await?;
    state.collector.watch(source).await?;
    Ok(id)
}

#[tauri::command]
pub async fn remove_source(source_id: String, state: State<'_, AppState>) -> Result<()> {
    state.collector.stop(&source_id);
    ll_core::db::queries::delete_source(&state.pool, &source_id).await?;
    Ok(())
}

#[tauri::command]
pub async fn list_sources(state: State<'_, AppState>) -> Result<Vec<LogSource>> {
    Ok(state.collector.list_sources())
}

#[tauri::command]
pub async fn watch_file(
    path: String,
    label: Option<String>,
    parser_hint: Option<String>,
    state: State<'_, AppState>,
) -> Result<String> {
    let lbl = label.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string()
    });
    let mut source = LogSource::new(lbl, LogSourceKind::File { path });
    source.parser_hint = parser_hint;
    let id = source.id.clone();
    ll_core::db::queries::insert_source(&state.pool, &source).await?;
    state.collector.watch(source).await?;
    Ok(id)
}

#[tauri::command]
pub async fn watch_docker(
    container_id: String,
    name: Option<String>,
    parser_hint: Option<String>,
    state: State<'_, AppState>,
) -> Result<String> {
    let label = name.clone().unwrap_or_else(|| container_id.chars().take(12).collect());
    let kind = LogSourceKind::DockerContainer {
        container_id: container_id.clone(),
        name: name.unwrap_or(container_id),
    };
    let mut source = LogSource::new(label, kind);
    source.parser_hint = parser_hint;
    let id = source.id.clone();
    ll_core::db::queries::insert_source(&state.pool, &source).await?;
    state.collector.watch(source).await?;
    Ok(id)
}

/// Opens a syslog listener.
///
/// The bind address is validated rather than passed through: a listener is
/// the one source in this application that accepts input from the network,
/// and binding it to the wrong interface exposes it beyond the operator's
/// own equipment.
#[tauri::command]
pub async fn watch_syslog(
    bind: String,
    protocol: Option<String>,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<String> {
    let bind = bind.trim().to_string();
    validate_bind(&bind)?;
    let protocol = match protocol.as_deref() {
        Some("tcp") => SyslogTransport::Tcp,
        Some("both") => SyslogTransport::Both,
        _ => SyslogTransport::Udp,
    };

    let label = label.unwrap_or_else(|| format!("syslog {bind}"));
    let source = LogSource::new(label, LogSourceKind::Syslog { bind, protocol });
    let id = source.id.clone();
    ll_core::db::queries::insert_source(&state.pool, &source).await?;
    state.collector.watch(source).await?;
    Ok(id)
}

/// Requires an explicit host and port, and refuses privileged ports, which
/// on macOS and Linux would need the whole application to run as root.
fn validate_bind(bind: &str) -> Result<()> {
    let address: std::net::SocketAddr = bind
        .parse()
        .map_err(|_| crate::error::LlError::Other(format!(
            "'{bind}' is not a host:port address, for example 0.0.0.0:5514"
        )))?;
    if address.port() < 1024 {
        return Err(crate::error::LlError::Other(format!(
            "port {} is privileged; use 5514 and forward 514 to it if the sender cannot be changed",
            address.port()
        )));
    }
    Ok(())
}
