use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::log_entry::{LogLevel, NormalizedEntry};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryFilter {
    pub text: Option<String>,
    pub levels: Option<Vec<LogLevel>>,
    pub services: Option<Vec<String>>,
    pub source_ids: Option<Vec<String>>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub pattern: Option<String>,
    pub has_stacktrace: Option<bool>,
    pub cluster_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub filter: QueryFilter,
    pub ai_query: Option<String>,
    pub highlight: bool,
    pub sort_desc: bool,
}

impl Default for QueryRequest {
    fn default() -> Self {
        Self {
            filter: QueryFilter { limit: Some(200), ..Default::default() },
            ai_query: None,
            highlight: true,
            sort_desc: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub entries: Vec<NormalizedEntry>,
    pub total: usize,
    pub took_ms: u64,
    pub highlights: std::collections::HashMap<String, Vec<String>>,
    pub ai_interpretation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub timestamp: DateTime<Utc>,
    pub total: u64,
    pub by_level: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCorrelation {
    pub service_a: String,
    pub service_b: String,
    pub correlation_score: f32,
    pub description: String,
}
