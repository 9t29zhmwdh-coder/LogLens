//! Azure OpenAI provider.
//!
//! Chosen over the public OpenAI endpoint because a network log carries
//! addresses, MAC addresses and host names from the operator's own estate.
//! On Azure that traffic stays inside a tenant the operator controls, which
//! is the difference between "an AI feature" and one that can be switched on
//! in an organization.

use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::Value;

use super::{prompts, response, AiAnalyzer};
use crate::models::analysis::{AiExplanation, AiSummary, RootCauseReport};
use crate::models::cluster::LogCluster;
use crate::models::log_entry::NormalizedEntry;

/// The last API version whose response shape this provider was written
/// against. Pinning it means a service-side rollout cannot change the
/// parsing behaviour without a code change.
const DEFAULT_API_VERSION: &str = "2024-10-21";

/// How the request authenticates.
pub enum AzureAuth {
    /// Resource key. Simple, but a long-lived secret.
    ApiKey(String),
    /// Entra ID access token, which is what a managed identity yields. The
    /// caller refreshes it; this provider only carries it.
    BearerToken(String),
}

pub struct AzureOpenAiAnalyzer {
    endpoint: String,
    deployment: String,
    api_version: String,
    auth: AzureAuth,
    client: Client,
}

impl AzureOpenAiAnalyzer {
    /// Fails on anything that is not an HTTPS Azure OpenAI endpoint, so a
    /// misconfigured host cannot quietly send the operator's log lines
    /// somewhere else.
    pub fn new(
        endpoint: impl Into<String>,
        deployment: impl Into<String>,
        auth: AzureAuth,
    ) -> Result<Self> {
        let endpoint = endpoint.into().trim_end_matches('/').to_string();
        validate_endpoint(&endpoint)?;
        let deployment = deployment.into();
        if deployment.is_empty() {
            return Err(anyhow!("Azure OpenAI deployment name must not be empty"));
        }
        Ok(Self {
            endpoint,
            deployment,
            api_version: DEFAULT_API_VERSION.to_string(),
            auth,
            client: Client::new(),
        })
    }

    /// Overrides the pinned API version for a resource that requires a
    /// different one.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    fn chat_url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, self.deployment, self.api_version
        )
    }

    async fn call(&self, prompt: &str) -> Result<Value> {
        let body = serde_json::json!({
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 2048,
            // The prompts ask for JSON; a low temperature keeps the reply
            // parseable instead of creatively formatted.
            "temperature": 0.2,
            "response_format": {"type": "json_object"}
        });

        let request = match &self.auth {
            AzureAuth::ApiKey(key) => self.client.post(self.chat_url()).header("api-key", key),
            AzureAuth::BearerToken(token) => self
                .client
                .post(self.chat_url())
                .header("Authorization", format!("Bearer {token}")),
        };

        let resp = request.json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            // The body can echo the prompt, which holds log content; only the
            // status is safe to surface.
            return Err(anyhow!("Azure OpenAI request failed with status {status}"));
        }
        Ok(resp.json().await?)
    }

    fn reply_text(resp: &Value) -> Option<&str> {
        resp["choices"][0]["message"]["content"].as_str()
    }

    fn tokens_used(resp: &Value) -> Option<u32> {
        resp["usage"]["total_tokens"].as_u64().map(|total| total as u32)
    }
}

/// Rejects plaintext and non-Azure hosts. Deliberately strict: the cost of
/// refusing a valid custom domain is a configuration error, the cost of
/// accepting a wrong one is leaked log data.
fn validate_endpoint(endpoint: &str) -> Result<()> {
    let Some(host) = endpoint.strip_prefix("https://") else {
        return Err(anyhow!("Azure OpenAI endpoint must use https"));
    };
    let host = host.split('/').next().unwrap_or_default();
    if host.is_empty() {
        return Err(anyhow!("Azure OpenAI endpoint has no host"));
    }
    if !(host.ends_with(".openai.azure.com") || host.ends_with(".cognitiveservices.azure.com")) {
        return Err(anyhow!(
            "Azure OpenAI endpoint must be an *.openai.azure.com or *.cognitiveservices.azure.com host, got {host}"
        ));
    }
    Ok(())
}

#[async_trait]
impl AiAnalyzer for AzureOpenAiAnalyzer {
    fn provider_name(&self) -> &str {
        "azure_openai"
    }

    fn model_name(&self) -> &str {
        &self.deployment
    }

    async fn explain_entry(&self, entry: &NormalizedEntry) -> Result<AiExplanation> {
        let resp = self.call(&prompts::explain_entry_prompt(entry)).await?;
        let text = Self::reply_text(&resp).ok_or_else(|| anyhow!("empty response"))?;
        let json = response::extract_json(text)?;
        Ok(response::explanation_from_json(&json, entry, self.provider_name(), self.model_name()))
    }

    async fn summarize_block(&self, entries: &[NormalizedEntry]) -> Result<AiSummary> {
        let resp = self.call(&prompts::summarize_block_prompt(entries)).await?;
        let text = Self::reply_text(&resp).ok_or_else(|| anyhow!("empty response"))?;
        let json = response::extract_json(text)?;
        Ok(response::summary_from_json(
            &json,
            entries,
            self.provider_name(),
            self.model_name(),
            Self::tokens_used(&resp),
        ))
    }

    async fn root_cause(&self, cluster: &LogCluster, samples: &[NormalizedEntry]) -> Result<RootCauseReport> {
        let resp = self.call(&prompts::root_cause_prompt(cluster, samples)).await?;
        let text = Self::reply_text(&resp).ok_or_else(|| anyhow!("empty response"))?;
        let json = response::extract_json(text)?;
        Ok(response::root_cause_from_json(&json, cluster, self.provider_name()))
    }

    async fn is_available(&self) -> bool {
        // A minimal completion is the only reliable probe: Azure has no
        // per-deployment health endpoint, and a deployment can exist in the
        // portal while returning 404 here.
        self.call("Reply with {}").await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyzer(endpoint: &str) -> Result<AzureOpenAiAnalyzer> {
        AzureOpenAiAnalyzer::new(endpoint, "gpt-4o", AzureAuth::ApiKey("k".into()))
    }

    #[test]
    fn a_valid_endpoint_is_accepted() {
        assert!(analyzer("https://contoso.openai.azure.com").is_ok());
        assert!(analyzer("https://contoso.cognitiveservices.azure.com").is_ok());
    }

    #[test]
    fn plaintext_transport_is_refused() {
        assert!(analyzer("http://contoso.openai.azure.com").is_err());
    }

    #[test]
    fn a_lookalike_host_is_refused() {
        // Would otherwise send every prompt, log content included, offsite.
        assert!(analyzer("https://contoso.openai.azure.com.evil.example").is_err());
        assert!(analyzer("https://api.openai.com").is_err());
    }

    #[test]
    fn an_empty_deployment_is_refused() {
        let result = AzureOpenAiAnalyzer::new(
            "https://contoso.openai.azure.com",
            "",
            AzureAuth::ApiKey("k".into()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn a_trailing_slash_does_not_double_up_in_the_url() {
        let a = analyzer("https://contoso.openai.azure.com/").unwrap();
        assert_eq!(
            a.chat_url(),
            "https://contoso.openai.azure.com/openai/deployments/gpt-4o/chat/completions?api-version=2024-10-21"
        );
    }

    #[test]
    fn the_api_version_can_be_overridden() {
        let a = analyzer("https://contoso.openai.azure.com").unwrap().with_api_version("2025-01-01");
        assert!(a.chat_url().ends_with("api-version=2025-01-01"));
    }

    #[test]
    fn the_deployment_name_is_reported_as_the_model() {
        assert_eq!(analyzer("https://contoso.openai.azure.com").unwrap().model_name(), "gpt-4o");
    }

    #[test]
    fn the_reply_is_read_from_the_chat_completion_shape() {
        let resp = serde_json::json!({
            "choices": [{"message": {"content": "{\"what\": \"ok\"}"}}],
            "usage": {"total_tokens": 128}
        });
        assert_eq!(AzureOpenAiAnalyzer::reply_text(&resp), Some("{\"what\": \"ok\"}"));
        assert_eq!(AzureOpenAiAnalyzer::tokens_used(&resp), Some(128));
    }
}
