use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::log_entry::LogLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogCluster {
    pub id: String,
    pub fingerprint: String,
    pub template: String,
    pub level: LogLevel,
    pub count: u64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub source_ids: Vec<String>,
    pub sample_ids: Vec<String>,
    pub services: Vec<String>,
    pub ai_summary: Option<String>,
}

impl LogCluster {
    pub fn new(fingerprint: impl Into<String>, template: impl Into<String>, level: LogLevel) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            fingerprint: fingerprint.into(),
            template: template.into(),
            level,
            count: 1,
            first_seen: now,
            last_seen: now,
            source_ids: Vec::new(),
            sample_ids: Vec::new(),
            services: Vec::new(),
            ai_summary: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStats {
    pub total_clusters: usize,
    pub total_entries: u64,
    pub top_errors: Vec<LogCluster>,
    pub by_level: std::collections::HashMap<String, u64>,
}
