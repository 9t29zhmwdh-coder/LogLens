use anyhow::Result;
use crate::models::log_entry::NormalizedEntry;
use crate::models::analysis::AiSummary;

pub fn export_json(entries: &[NormalizedEntry], summary: Option<&AiSummary>) -> Result<String> {
    let payload = serde_json::json!({
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "entry_count": entries.len(),
        "ai_summary": summary,
        "entries": entries,
    });
    Ok(serde_json::to_string_pretty(&payload)?)
}
