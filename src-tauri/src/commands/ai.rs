use tauri::State;
use ll_core::ai::{AiAnalyzer, claude::ClaudeAnalyzer, ollama::OllamaAnalyzer};
use ll_core::models::log_entry::NormalizedEntry;
use ll_core::models::analysis::{AiExplanation, AiSummary, RootCauseReport};
use crate::state::AppState;
use crate::error::{Result, LlError};

async fn make_analyzer(state: &AppState) -> Result<Box<dyn AiAnalyzer>> {
    let settings = state.settings.read().await.clone();
    if settings.ai_backend == "ollama" {
        return Ok(Box::new(OllamaAnalyzer::new(settings.ollama_url, settings.ollama_model)));
    }
    let key = keyring::Entry::new("loglens", "claude_api_key")?
        .get_password()?;
    if key.is_empty() {
        return Err(LlError::Other("No API key configured".to_string()));
    }
    Ok(Box::new(ClaudeAnalyzer::new(key)))
}

#[tauri::command]
pub async fn explain_entry(entry: NormalizedEntry, state: State<'_, AppState>) -> Result<AiExplanation> {
    let ai = make_analyzer(&state).await?;
    let expl = ai.explain_entry(&entry).await?;
    ll_core::db::queries::insert_explanation(&state.pool, &expl).await?;
    Ok(expl)
}

#[tauri::command]
pub async fn get_cached_explanation(entry_id: String, state: State<'_, AppState>) -> Result<Option<AiExplanation>> {
    Ok(ll_core::db::queries::get_explanation_for_entry(&state.pool, &entry_id).await?)
}

#[tauri::command]
pub async fn summarize_entries(entries: Vec<NormalizedEntry>, state: State<'_, AppState>) -> Result<AiSummary> {
    if entries.is_empty() {
        return Err(LlError::Other("No entries to summarize".to_string()));
    }
    let ai = make_analyzer(&state).await?;
    Ok(ai.summarize_block(&entries).await?)
}

#[tauri::command]
pub async fn analyze_cluster(cluster_id: String, state: State<'_, AppState>) -> Result<RootCauseReport> {
    let clusters = ll_core::db::queries::list_clusters(&state.pool, 1000).await?;
    let cluster = clusters.into_iter()
        .find(|c| c.id == cluster_id)
        .ok_or_else(|| LlError::Other(format!("Cluster not found: {}", cluster_id)))?;

    // Fetch sample entries for context
    let req = ll_core::models::query::QueryRequest {
        filter: ll_core::models::query::QueryFilter {
            cluster_id: Some(cluster_id),
            limit: Some(5),
            ..Default::default()
        },
        ..Default::default()
    };
    let engine = ll_core::query::QueryEngine::new(state.pool.clone());
    let result = engine.query(&req).await?;

    let ai = make_analyzer(&state).await?;
    Ok(ai.root_cause(&cluster, &result.entries).await?)
}
