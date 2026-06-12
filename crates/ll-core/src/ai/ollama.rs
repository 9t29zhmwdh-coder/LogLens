use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::Client;
use crate::models::log_entry::NormalizedEntry;
use crate::models::cluster::LogCluster;
use crate::models::analysis::{AiExplanation, AiSummary, RootCauseReport};
use super::AiAnalyzer;
use super::prompts;
use super::claude::ClaudeAnalyzer; // reuse JSON parsers via delegation pattern
use uuid::Uuid;
use chrono::Utc;

pub struct OllamaAnalyzer {
    base_url: String,
    model: String,
    client: Client,
}

impl OllamaAnalyzer {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            client: Client::new(),
        }
    }

    async fn call(&self, prompt: &str) -> Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "format": "json"
        });

        let resp = self.client
            .post(format!("{}/api/generate", self.base_url.trim_end_matches('/')))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Ollama error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(json["response"].as_str().unwrap_or("{}").to_string())
    }

    fn parse(text: &str) -> Result<serde_json::Value> {
        let start = text.find('{').ok_or_else(|| anyhow!("No JSON"))?;
        let end = text.rfind('}').ok_or_else(|| anyhow!("No JSON end"))?;
        Ok(serde_json::from_str(&text[start..=end])?)
    }
}

#[async_trait]
impl AiAnalyzer for OllamaAnalyzer {
    fn provider_name(&self) -> &str { "ollama" }
    fn model_name(&self) -> &str { &self.model }

    async fn explain_entry(&self, entry: &NormalizedEntry) -> Result<AiExplanation> {
        let prompt = prompts::explain_entry_prompt(entry);
        let text = self.call(&prompt).await?;
        let j = Self::parse(&text)?;

        Ok(AiExplanation {
            id: Uuid::new_v4().to_string(),
            entry_id: entry.id.clone(),
            created_at: Utc::now(),
            what: j["what"].as_str().unwrap_or("").to_string(),
            why: j["why"].as_str().unwrap_or("").to_string(),
            impact: j["impact"].as_str().unwrap_or("").to_string(),
            debug_steps: json_str_arr(&j["debug_steps"]),
            possible_causes: json_str_arr(&j["possible_causes"]),
            fix_suggestions: json_str_arr(&j["fix_suggestions"]),
            confidence: j["confidence"].as_f64().unwrap_or(0.5) as f32,
            ai_provider: self.provider_name().to_string(),
            model: self.model_name().to_string(),
        })
    }

    async fn summarize_block(&self, entries: &[NormalizedEntry]) -> Result<AiSummary> {
        let prompt = prompts::summarize_block_prompt(entries);
        let text = self.call(&prompt).await?;
        let j = Self::parse(&text)?;

        let first_ts = entries.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let last_ts = entries.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let mut dist = std::collections::HashMap::new();
        for e in entries {
            *dist.entry(format!("{:?}", e.level)).or_insert(0u64) += 1;
        }

        Ok(AiSummary {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            entry_count: entries.len(),
            time_range_start: first_ts,
            time_range_end: last_ts,
            overview: j["overview"].as_str().unwrap_or("").to_string(),
            key_issues: json_str_arr(&j["key_issues"]),
            patterns: json_str_arr(&j["patterns"]),
            root_causes: json_str_arr(&j["root_causes"]),
            recommendations: json_str_arr(&j["recommendations"]),
            severity_distribution: dist,
            ai_provider: self.provider_name().to_string(),
            model: self.model_name().to_string(),
            tokens_used: None,
        })
    }

    async fn root_cause(&self, cluster: &LogCluster, samples: &[NormalizedEntry]) -> Result<RootCauseReport> {
        let prompt = prompts::root_cause_prompt(cluster, samples);
        let text = self.call(&prompt).await?;
        let j = Self::parse(&text)?;

        use crate::models::analysis::FixStep;
        let fix_steps: Vec<FixStep> = j["fix_suggestions"].as_array()
            .map(|arr| arr.iter().enumerate().map(|(i, s)| FixStep {
                step: s["step"].as_u64().unwrap_or(i as u64 + 1) as u8,
                title: s["title"].as_str().unwrap_or("").to_string(),
                description: s["description"].as_str().unwrap_or("").to_string(),
                command: s["command"].as_str().map(str::to_string),
                code: s["code"].as_str().map(str::to_string),
            }).collect())
            .unwrap_or_default();

        Ok(RootCauseReport {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            trigger_entry_id: None,
            cluster_id: Some(cluster.id.clone()),
            title: j["title"].as_str().unwrap_or("Root Cause Analysis").to_string(),
            root_cause: j["root_cause"].as_str().unwrap_or("").to_string(),
            evidence: json_str_arr(&j["evidence"]),
            contributing_factors: json_str_arr(&j["contributing_factors"]),
            fix_suggestions: fix_steps,
            confidence: j["confidence"].as_f64().unwrap_or(0.5) as f32,
            ai_provider: self.provider_name().to_string(),
        })
    }

    async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url.trim_end_matches('/')))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

fn json_str_arr(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}
