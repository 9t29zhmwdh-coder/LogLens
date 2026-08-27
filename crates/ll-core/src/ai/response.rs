//! Turning a model's JSON reply into the crate's analysis types.
//!
//! Every provider is asked for the same JSON shape, so the conversion belongs
//! here rather than in each provider. Missing fields degrade to empty values
//! instead of failing the whole analysis: a report with four of five sections
//! is still useful, an error is not.

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::models::analysis::{AiExplanation, AiSummary, FixStep, RootCauseReport};
use crate::models::cluster::LogCluster;
use crate::models::log_entry::NormalizedEntry;

/// Models like to wrap JSON in prose or a fenced block. Taking the outermost
/// braces is what survives both.
pub fn extract_json(text: &str) -> Result<Value> {
    let start = text.find('{').ok_or_else(|| anyhow!("no JSON object in response"))?;
    let end = text.rfind('}').ok_or_else(|| anyhow!("unterminated JSON object in response"))?;
    if end < start {
        return Err(anyhow!("malformed JSON object in response"));
    }
    Ok(serde_json::from_str(&text[start..=end])?)
}

pub fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(|i| i.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_string()
}

/// Confidence is clamped because a model asked for 0.0 to 1.0 will
/// occasionally answer 95, and an out-of-range value would misrender in the
/// interface rather than fail loudly.
fn confidence(value: &Value) -> f32 {
    (value["confidence"].as_f64().unwrap_or(0.5) as f32).clamp(0.0, 1.0)
}

pub fn explanation_from_json(
    json: &Value,
    entry: &NormalizedEntry,
    provider: &str,
    model: &str,
) -> AiExplanation {
    AiExplanation {
        id: Uuid::new_v4().to_string(),
        entry_id: entry.id.clone(),
        created_at: Utc::now(),
        what: text(json, "what"),
        why: text(json, "why"),
        impact: text(json, "impact"),
        debug_steps: string_array(&json["debug_steps"]),
        possible_causes: string_array(&json["possible_causes"]),
        fix_suggestions: string_array(&json["fix_suggestions"]),
        confidence: confidence(json),
        ai_provider: provider.to_string(),
        model: model.to_string(),
    }
}

pub fn summary_from_json(
    json: &Value,
    entries: &[NormalizedEntry],
    provider: &str,
    model: &str,
    tokens_used: Option<u32>,
) -> AiSummary {
    let mut severity_distribution = std::collections::HashMap::new();
    for entry in entries {
        *severity_distribution.entry(format!("{:?}", entry.level)).or_insert(0u64) += 1;
    }

    AiSummary {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now(),
        entry_count: entries.len(),
        time_range_start: entries.first().map(|e| e.timestamp).unwrap_or_else(Utc::now),
        time_range_end: entries.last().map(|e| e.timestamp).unwrap_or_else(Utc::now),
        overview: text(json, "overview"),
        key_issues: string_array(&json["key_issues"]),
        patterns: string_array(&json["patterns"]),
        root_causes: string_array(&json["root_causes"]),
        recommendations: string_array(&json["recommendations"]),
        severity_distribution,
        ai_provider: provider.to_string(),
        model: model.to_string(),
        tokens_used,
    }
}

pub fn root_cause_from_json(
    json: &Value,
    cluster: &LogCluster,
    provider: &str,
) -> RootCauseReport {
    RootCauseReport {
        id: Uuid::new_v4().to_string(),
        created_at: Utc::now(),
        trigger_entry_id: None,
        cluster_id: Some(cluster.id.clone()),
        title: json["title"].as_str().unwrap_or("Root Cause Analysis").to_string(),
        root_cause: text(json, "root_cause"),
        evidence: string_array(&json["evidence"]),
        contributing_factors: string_array(&json["contributing_factors"]),
        fix_suggestions: fix_steps(&json["fix_suggestions"]),
        confidence: confidence(json),
        ai_provider: provider.to_string(),
    }
}

/// Numbers the steps from their position when the model omits the field, so
/// the interface always has an ordering to render.
fn fix_steps(value: &Value) -> Vec<FixStep> {
    value
        .as_array()
        .map(|steps| {
            steps
                .iter()
                .enumerate()
                .map(|(index, step)| FixStep {
                    step: step["step"].as_u64().unwrap_or(index as u64 + 1) as u8,
                    title: step["title"].as_str().unwrap_or("").to_string(),
                    description: step["description"].as_str().unwrap_or("").to_string(),
                    command: step["command"].as_str().map(str::to_string),
                    code: step["code"].as_str().map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_recovered_from_surrounding_prose() {
        let reply = "Here is the analysis:\n```json\n{\"what\": \"a failure\"}\n```\nHope that helps.";
        assert_eq!(extract_json(reply).unwrap()["what"], "a failure");
    }

    #[test]
    fn a_reply_without_json_is_an_error_not_a_panic() {
        assert!(extract_json("I could not analyze this.").is_err());
        assert!(extract_json("").is_err());
        assert!(extract_json("} backwards {").is_err());
    }

    #[test]
    fn an_out_of_range_confidence_is_clamped() {
        assert_eq!(confidence(&serde_json::json!({"confidence": 95})), 1.0);
        assert_eq!(confidence(&serde_json::json!({"confidence": -1})), 0.0);
        assert_eq!(confidence(&serde_json::json!({"confidence": 0.7})), 0.7);
    }

    #[test]
    fn a_missing_confidence_falls_back_to_the_midpoint() {
        assert_eq!(confidence(&serde_json::json!({})), 0.5);
    }

    #[test]
    fn steps_without_numbers_are_numbered_by_position() {
        let steps = fix_steps(&serde_json::json!([
            {"title": "first"},
            {"title": "second", "step": 9}
        ]));
        assert_eq!(steps[0].step, 1);
        assert_eq!(steps[1].step, 9);
    }

    #[test]
    fn a_non_array_field_yields_an_empty_list_rather_than_failing() {
        assert!(string_array(&serde_json::json!("not an array")).is_empty());
        assert!(string_array(&serde_json::Value::Null).is_empty());
        assert!(fix_steps(&serde_json::json!({"not": "an array"})).is_empty());
    }

    #[test]
    fn mixed_type_arrays_keep_only_their_strings() {
        assert_eq!(string_array(&serde_json::json!(["a", 1, null, "b"])), vec!["a", "b"]);
    }
}
