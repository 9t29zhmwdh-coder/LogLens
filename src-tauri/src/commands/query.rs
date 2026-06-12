use tauri::State;
use ll_core::query::QueryEngine;
use ll_core::models::query::{QueryRequest, QueryResult, TimelineBucket};
use crate::state::AppState;
use crate::error::Result;

#[tauri::command]
pub async fn query_logs(req: QueryRequest, state: State<'_, AppState>) -> Result<QueryResult> {
    let engine = QueryEngine::new(state.pool.clone());
    Ok(engine.query(&req).await?)
}

#[tauri::command]
pub async fn get_timeline(
    from: String,
    to: String,
    bucket_minutes: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<TimelineBucket>> {
    let engine = QueryEngine::new(state.pool.clone());
    Ok(engine.timeline(&from, &to, bucket_minutes.unwrap_or(5)).await?)
}

#[tauri::command]
pub async fn translate_query(nl_query: String, state: State<'_, AppState>) -> Result<ll_core::models::query::QueryFilter> {
    let settings = state.settings.read().await.clone();
    let (api_key, model) = if settings.ai_backend == "ollama" {
        (settings.ollama_url.clone(), settings.ollama_model.clone())
    } else {
        let key = keyring::Entry::new("loglens", "claude_api_key")
            .ok()
            .and_then(|e| e.get_password().ok())
            .unwrap_or_default();
        (key, "claude-haiku-4-5-20251001".to_string())
    };

    Ok(ll_core::query::ai_query::translate(&nl_query, &api_key, &api_key, &model).await)
}
