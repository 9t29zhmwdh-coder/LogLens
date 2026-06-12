use tauri::State;
use ll_core::models::cluster::{LogCluster, ClusterStats};
use crate::state::AppState;
use crate::error::Result;

#[tauri::command]
pub async fn list_clusters(limit: Option<i64>, state: State<'_, AppState>) -> Result<Vec<LogCluster>> {
    Ok(ll_core::db::queries::list_clusters(&state.pool, limit.unwrap_or(50)).await?)
}

#[tauri::command]
pub async fn get_cluster_stats(state: State<'_, AppState>) -> Result<ClusterStats> {
    let clusters = ll_core::db::queries::list_clusters(&state.pool, 1000).await?;
    let top_errors = state.grouper.top_errors(10);

    let total_entries = clusters.iter().map(|c| c.count).sum();
    let mut by_level = std::collections::HashMap::new();
    for c in &clusters {
        *by_level.entry(format!("{:?}", c.level)).or_insert(0u64) += c.count;
    }

    Ok(ClusterStats {
        total_clusters: clusters.len(),
        total_entries,
        top_errors,
        by_level,
    })
}
