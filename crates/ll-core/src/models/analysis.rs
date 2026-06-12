use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiExplanation {
    pub id: String,
    pub entry_id: String,
    pub created_at: DateTime<Utc>,
    pub what: String,
    pub why: String,
    pub impact: String,
    pub debug_steps: Vec<String>,
    pub possible_causes: Vec<String>,
    pub fix_suggestions: Vec<String>,
    pub confidence: f32,
    pub ai_provider: String,
    pub model: String,
}

impl AiExplanation {
    pub fn new(entry_id: impl Into<String>, provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            entry_id: entry_id.into(),
            created_at: Utc::now(),
            what: String::new(),
            why: String::new(),
            impact: String::new(),
            debug_steps: Vec::new(),
            possible_causes: Vec::new(),
            fix_suggestions: Vec::new(),
            confidence: 0.0,
            ai_provider: provider.into(),
            model: model.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSummary {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub entry_count: usize,
    pub time_range_start: DateTime<Utc>,
    pub time_range_end: DateTime<Utc>,
    pub overview: String,
    pub key_issues: Vec<String>,
    pub patterns: Vec<String>,
    pub root_causes: Vec<String>,
    pub recommendations: Vec<String>,
    pub severity_distribution: std::collections::HashMap<String, u64>,
    pub ai_provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseReport {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub trigger_entry_id: Option<String>,
    pub cluster_id: Option<String>,
    pub title: String,
    pub root_cause: String,
    pub evidence: Vec<String>,
    pub contributing_factors: Vec<String>,
    pub fix_suggestions: Vec<FixStep>,
    pub confidence: f32,
    pub ai_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixStep {
    pub step: u8,
    pub title: String,
    pub description: String,
    pub command: Option<String>,
    pub code: Option<String>,
}
