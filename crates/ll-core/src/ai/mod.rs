pub mod claude;
pub mod ollama;
pub mod prompts;

use async_trait::async_trait;
use anyhow::Result;
use crate::models::log_entry::NormalizedEntry;
use crate::models::cluster::LogCluster;
use crate::models::analysis::{AiExplanation, AiSummary, RootCauseReport};

#[async_trait]
pub trait AiAnalyzer: Send + Sync {
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;

    async fn explain_entry(&self, entry: &NormalizedEntry) -> Result<AiExplanation>;
    async fn summarize_block(&self, entries: &[NormalizedEntry]) -> Result<AiSummary>;
    async fn root_cause(&self, cluster: &LogCluster, samples: &[NormalizedEntry]) -> Result<RootCauseReport>;
    async fn is_available(&self) -> bool;
}
