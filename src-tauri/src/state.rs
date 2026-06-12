use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use sqlx::SqlitePool;
use ll_core::collector::LogCollector;
use ll_core::clustering::ClusterGrouper;
use ll_core::models::log_entry::NormalizedEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub ai_backend: String,      // "claude" | "ollama"
    pub ollama_url: String,
    pub ollama_model: String,
    pub theme: String,
    pub max_entries_in_memory: usize,
    pub auto_cluster: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            ai_backend: "claude".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "llama3".to_string(),
            theme: "dark".to_string(),
            max_entries_in_memory: 10_000,
            auto_cluster: true,
        }
    }
}

pub struct AppState {
    pub pool: SqlitePool,
    pub collector: Arc<LogCollector>,
    pub grouper: Arc<ClusterGrouper>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub log_tx: mpsc::Sender<NormalizedEntry>,
}
