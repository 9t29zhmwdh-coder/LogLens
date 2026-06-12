use tauri::State;
use crate::state::{AppState, AppSettings};
use crate::error::Result;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn save_settings(settings: AppSettings, state: State<'_, AppState>) -> Result<()> {
    let json = serde_json::to_string(&settings).map_err(|e| crate::error::LlError::Other(e.to_string()))?;
    ll_core::db::queries::set_setting(&state.pool, "app_settings", &json).await?;
    *state.settings.write().await = settings;
    Ok(())
}

#[tauri::command]
pub async fn save_api_key(key: String) -> Result<()> {
    keyring::Entry::new("loglens", "claude_api_key")?.set_password(&key)?;
    Ok(())
}

#[tauri::command]
pub async fn has_api_key() -> bool {
    keyring::Entry::new("loglens", "claude_api_key")
        .ok()
        .and_then(|e| e.get_password().ok())
        .map(|k| !k.is_empty())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn check_ai_backend(state: State<'_, AppState>) -> Result<bool> {
    let settings = state.settings.read().await.clone();
    let available = if settings.ai_backend == "ollama" {
        let analyzer = ll_core::ai::ollama::OllamaAnalyzer::new(&settings.ollama_url, &settings.ollama_model);
        ll_core::ai::AiAnalyzer::is_available(&analyzer).await
    } else {
        let key = keyring::Entry::new("loglens", "claude_api_key")
            .ok()
            .and_then(|e| e.get_password().ok())
            .unwrap_or_default();
        let analyzer = ll_core::ai::claude::ClaudeAnalyzer::new(key);
        ll_core::ai::AiAnalyzer::is_available(&analyzer).await
    };
    Ok(available)
}
