use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::Value;
use crate::models::log_entry::NormalizedEntry;
use crate::models::cluster::LogCluster;
use crate::models::analysis::{AiExplanation, AiSummary, RootCauseReport};
use super::AiAnalyzer;
use super::prompts;
use super::response;

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
}

#[async_trait]
impl AiAnalyzer for ClaudeAnalyzer {
    fn provider_name(&self) -> &str { "claude" }
    fn model_name(&self) -> &str { MODEL }

    async fn explain_entry(&self, entry: &NormalizedEntry) -> Result<AiExplanation> {
        let prompt = prompts::explain_entry_prompt(entry);
        let resp = self.call(&prompt).await?;
        let text = Self::extract_json_text(&resp).ok_or_else(|| anyhow!("Empty response"))?;
        let j = response::extract_json(text)?;
        Ok(response::explanation_from_json(&j, entry, self.provider_name(), self.model_name()))
    }

    async fn summarize_block(&self, entries: &[NormalizedEntry]) -> Result<AiSummary> {
        let prompt = prompts::summarize_block_prompt(entries);
        let resp = self.call(&prompt).await?;
        let text = Self::extract_json_text(&resp).ok_or_else(|| anyhow!("Empty response"))?;
        let j = response::extract_json(text)?;
        let tokens_used = resp["usage"]["input_tokens"].as_u64().and_then(|input| {
            resp["usage"]["output_tokens"].as_u64().map(|output| (input + output) as u32)
        });
        Ok(response::summary_from_json(&j, entries, self.provider_name(), self.model_name(), tokens_used))
    }

    async fn root_cause(&self, cluster: &LogCluster, samples: &[NormalizedEntry]) -> Result<RootCauseReport> {
        let prompt = prompts::root_cause_prompt(cluster, samples);
        let resp = self.call(&prompt).await?;
        let text = Self::extract_json_text(&resp).ok_or_else(|| anyhow!("Empty response"))?;
        let j = response::extract_json(text)?;
        Ok(response::root_cause_from_json(&j, cluster, self.provider_name()))
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

