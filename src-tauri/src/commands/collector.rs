use tauri::State;
use ll_core::models::log_entry::{LogSource, LogSourceKind};
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
pub async fn watch_file(path: String, label: Option<String>, state: State<'_, AppState>) -> Result<String> {
    let lbl = label.unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string()
    });
    let source = LogSource::new(lbl, LogSourceKind::File { path });
    let id = source.id.clone();
    ll_core::db::queries::insert_source(&state.pool, &source).await?;
    state.collector.watch(source).await?;
    Ok(id)
}

#[tauri::command]
pub async fn watch_docker(container_id: String, name: Option<String>, state: State<'_, AppState>) -> Result<String> {
    let label = name.clone().unwrap_or_else(|| container_id.chars().take(12).collect());
    let kind = LogSourceKind::DockerContainer {
        container_id: container_id.clone(),
        name: name.unwrap_or(container_id),
    };
    let source = LogSource::new(label, kind);
    let id = source.id.clone();
    ll_core::db::queries::insert_source(&state.pool, &source).await?;
    state.collector.watch(source).await?;
    Ok(id)
}
