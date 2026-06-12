use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::Value;
use crate::models::log_entry::NormalizedEntry;
use crate::models::cluster::LogCluster;
use crate::models::analysis::{AiExplanation, AiSummary, RootCauseReport, FixStep};
use super::AiAnalyzer;
use super::prompts;
use chrono::Utc;
use uuid::Uuid;

const CLAUDE_API: &str = "https://api.anthropic.com";
const MODEL: &str = "claude-haiku-4-5-20251001";

pub struct ClaudeAnalyzer {
    api_key: String,
    client: Client,
}

impl ClaudeAnalyzer {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
        }
    }

    async fn call(&self, prompt: &str) -> Result<Value> {
        let body = serde_json::json!({
            "model": MODEL,
            "max_tokens": 2048,
            "messages": [{"role": "user", "content": prompt}]
        });

        let resp = self.client
            .post(format!("{}/v1/messages", CLAUDE_API))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Claude API error {}: {}", status, text));
        }

        let json: Value = resp.json().await?;
        Ok(json)
    }

    fn extract_json_text(resp: &Value) -> Option<&str> {
        resp["content"][0]["text"].as_str()
    }

    fn parse_json_from_text(text: &str) -> Result<Value> {
        let start = text.find('{').ok_or_else(|| anyhow!("No JSON in response"))?;
        let end = text.rfind('}').ok_or_else(|| anyhow!("No JSON end in response"))?;
        Ok(serde_json::from_str(&text[start..=end])?)
    }
}

#[async_trait]
impl AiAnalyzer for ClaudeAnalyzer {
    fn provider_name(&self) -> &str { "claude" }
    fn model_name(&self) -> &str { MODEL }

    async fn explain_entry(&self, entry: &NormalizedEntry) -> Result<AiExplanation> {
        let prompt = prompts::explain_entry_prompt(entry);
        let resp = self.call(&prompt).await?;
        let text = Self::extract_json_text(&resp).ok_or_else(|| anyhow!("Empty response"))?;
        let j = Self::parse_json_from_text(text)?;

        Ok(AiExplanation {
            id: Uuid::new_v4().to_string(),
            entry_id: entry.id.clone(),
            created_at: Utc::now(),
            what: j["what"].as_str().unwrap_or("").to_string(),
            why: j["why"].as_str().unwrap_or("").to_string(),
            impact: j["impact"].as_str().unwrap_or("").to_string(),
            debug_steps: parse_str_array(&j["debug_steps"]),
            possible_causes: parse_str_array(&j["possible_causes"]),
            fix_suggestions: parse_str_array(&j["fix_suggestions"]),
            confidence: j["confidence"].as_f64().unwrap_or(0.5) as f32,
            ai_provider: self.provider_name().to_string(),
            model: self.model_name().to_string(),
        })
    }

    async fn summarize_block(&self, entries: &[NormalizedEntry]) -> Result<AiSummary> {
        let prompt = prompts::summarize_block_prompt(entries);
        let resp = self.call(&prompt).await?;
        let text = Self::extract_json_text(&resp).ok_or_else(|| anyhow!("Empty response"))?;
        let j = Self::parse_json_from_text(text)?;

        let first_ts = entries.first().map(|e| e.timestamp).unwrap_or_else(Utc::now);
        let last_ts = entries.last().map(|e| e.timestamp).unwrap_or_else(Utc::now);

        let mut severity_dist = std::collections::HashMap::new();
        for e in entries {
            *severity_dist.entry(format!("{:?}", e.level)).or_insert(0u64) += 1;
        }

        Ok(AiSummary {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            entry_count: entries.len(),
            time_range_start: first_ts,
            time_range_end: last_ts,
            overview: j["overview"].as_str().unwrap_or("").to_string(),
            key_issues: parse_str_array(&j["key_issues"]),
            patterns: parse_str_array(&j["patterns"]),
            root_causes: parse_str_array(&j["root_causes"]),
            recommendations: parse_str_array(&j["recommendations"]),
            severity_distribution: severity_dist,
            ai_provider: self.provider_name().to_string(),
            model: self.model_name().to_string(),
            tokens_used: resp["usage"]["input_tokens"].as_u64()
                .and_then(|i| resp["usage"]["output_tokens"].as_u64().map(|o| (i + o) as u32)),
        })
    }

    async fn root_cause(&self, cluster: &LogCluster, samples: &[NormalizedEntry]) -> Result<RootCauseReport> {
        let prompt = prompts::root_cause_prompt(cluster, samples);
        let resp = self.call(&prompt).await?;
        let text = Self::extract_json_text(&resp).ok_or_else(|| anyhow!("Empty response"))?;
        let j = Self::parse_json_from_text(text)?;

        let fix_steps: Vec<FixStep> = j["fix_suggestions"].as_array()
            .map(|arr| arr.iter().enumerate().map(|(i, step)| FixStep {
                step: step["step"].as_u64().unwrap_or(i as u64 + 1) as u8,
                title: step["title"].as_str().unwrap_or("").to_string(),
                description: step["description"].as_str().unwrap_or("").to_string(),
                command: step["command"].as_str().map(str::to_string),
                code: step["code"].as_str().map(str::to_string),
            }).collect())
            .unwrap_or_default();

        Ok(RootCauseReport {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            trigger_entry_id: None,
            cluster_id: Some(cluster.id.clone()),
            title: j["title"].as_str().unwrap_or("Root Cause Analysis").to_string(),
            root_cause: j["root_cause"].as_str().unwrap_or("").to_string(),
            evidence: parse_str_array(&j["evidence"]),
            contributing_factors: parse_str_array(&j["contributing_factors"]),
            fix_suggestions: fix_steps,
            confidence: j["confidence"].as_f64().unwrap_or(0.5) as f32,
            ai_provider: self.provider_name().to_string(),
        })
    }

    async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/v1/models", CLAUDE_API))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

fn parse_str_array(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|arr| arr.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}
