use crate::models::log_entry::NormalizedEntry;
use crate::models::cluster::LogCluster;

/// Fields a network classifier may have attached, in the order an operator
/// would read them: who, then where, then against what.
const NETWORK_FIELDS: [&str; 12] = [
    "event_type", "vendor", "client_mac", "src_ip", "dst_ip", "src_port",
    "dst_port", "protocol", "interface", "ssid", "rule_id", "reason",
];

/// Renders the network entities a classifier extracted, if any.
///
/// Without this the model sees only the vendor's own wording, which is often
/// the least informative part of the line: hostapd says "invalid MIC in msg
/// 2/4", never "wrong WiFi password".
fn network_context(entry: &NormalizedEntry) -> String {
    let Some(fields) = entry.fields.as_object() else {
        return String::new();
    };
    if !fields.contains_key("event_type") {
        return String::new();
    }
    let lines: Vec<String> = NETWORK_FIELDS
        .iter()
        .filter_map(|key| fields.get(*key).and_then(|v| v.as_str()).map(|v| format!("- {key}: {v}")))
        .collect();
    format!(
        "\n\nThis is a network device log. Classified entities:\n{}\n\nAnswer as a network engineer: name the device, client and rule involved, and say what an operator should check on the equipment.",
        lines.join("\n")
    )
}

pub fn explain_entry_prompt(entry: &NormalizedEntry) -> String {
    let stacktrace = entry.stacktrace.as_ref()
        .map(|lines| format!("\nStacktrace:\n{}", lines.join("\n")))
        .unwrap_or_default();
    let network = network_context(entry);

    format!(
        r#"You are a senior software engineer analyzing a log entry.

Log entry:
- Timestamp: {}
- Level: {:?}
- Service: {}
- Message: {}{}
- Raw: {}{}

Analyze this log entry and respond ONLY with a JSON object:
{{
  "what": "Brief description of what happened",
  "why": "Likely reason this occurred",
  "impact": "Potential impact on the system",
  "debug_steps": ["step 1", "step 2"],
  "possible_causes": ["cause 1", "cause 2"],
  "fix_suggestions": ["fix 1", "fix 2"],
  "confidence": 0.85
}}"#,
        entry.timestamp.to_rfc3339(),
        entry.level,
        entry.service.as_deref().unwrap_or("unknown"),
        entry.message,
        stacktrace,
        entry.raw.chars().take(500).collect::<String>(),
        network,
    )
}

pub fn summarize_block_prompt(entries: &[NormalizedEntry]) -> String {
    let first = entries.first().map(|e| e.timestamp.to_rfc3339()).unwrap_or_default();
    let last = entries.last().map(|e| e.timestamp.to_rfc3339()).unwrap_or_default();

    let sample: String = entries.iter().take(20)
        .map(|e| format!("[{:?}] {} {}", e.level, e.service.as_deref().unwrap_or("-"), e.message))
        .collect::<Vec<_>>()
        .join("\n");

    let level_counts = count_levels(entries);

    format!(
        r#"You are a senior DevOps engineer summarizing a log block.

Time range: {} → {}
Entry count: {}
Level distribution: {:?}

Sample entries (up to 20):
{}

Respond ONLY with JSON:
{{
  "overview": "High-level summary of what happened",
  "key_issues": ["issue 1", "issue 2"],
  "patterns": ["pattern 1", "pattern 2"],
  "root_causes": ["cause 1"],
  "recommendations": ["action 1", "action 2"]
}}"#,
        first, last, entries.len(), level_counts, sample
    )
}

pub fn root_cause_prompt(cluster: &LogCluster, samples: &[NormalizedEntry]) -> String {
    let sample_msgs: String = samples.iter().take(5)
        .map(|e| format!("- {}", e.message))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a senior engineer performing root cause analysis.

Error cluster:
- Template: {}
- Occurrences: {}
- Level: {:?}
- Services affected: {}
- First seen: {}
- Last seen: {}

Sample messages:
{}

Respond ONLY with JSON:
{{
  "title": "Short incident title",
  "root_cause": "Root cause explanation",
  "evidence": ["evidence 1", "evidence 2"],
  "contributing_factors": ["factor 1"],
  "fix_suggestions": [
    {{
      "step": 1,
      "title": "Fix title",
      "description": "What to do",
      "command": "optional shell command",
      "code": "optional code snippet"
    }}
  ],
  "confidence": 0.8
}}"#,
        cluster.template,
        cluster.count,
        cluster.level,
        cluster.services.join(", "),
        cluster.first_seen.to_rfc3339(),
        cluster.last_seen.to_rfc3339(),
        sample_msgs,
    )
}

fn count_levels(entries: &[NormalizedEntry]) -> std::collections::HashMap<String, usize> {
    let mut map = std::collections::HashMap::new();
    for e in entries {
        *map.entry(format!("{:?}", e.level)).or_insert(0) += 1;
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::log_entry::{LogFormat, LogLevel};
    use chrono::Utc;

    fn entry(fields: serde_json::Value) -> NormalizedEntry {
        NormalizedEntry {
            id: "id".into(),
            source_id: "src".into(),
            source_label: "label".into(),
            timestamp: Utc::now(),
            level: LogLevel::Warn,
            service: Some("hostapd".into()),
            message: "WPA: invalid MIC in msg 2/4 of 4-Way Handshake".into(),
            stacktrace: None,
            fields,
            raw: "raw line".into(),
            format: LogFormat::Syslog,
            fingerprint: "fp".into(),
            cluster_id: None,
            ingested_at: Utc::now(),
        }
    }

    #[test]
    fn a_classified_entry_gets_the_network_engineer_framing() {
        let prompt = explain_entry_prompt(&entry(serde_json::json!({
            "event_type": "wifi_auth_failure",
            "vendor": "unifi",
            "client_mac": "aa:bb:cc:dd:ee:ff",
            "interface": "ath0"
        })));
        assert!(prompt.contains("network device log"));
        assert!(prompt.contains("- event_type: wifi_auth_failure"));
        assert!(prompt.contains("- client_mac: aa:bb:cc:dd:ee:ff"));
        assert!(prompt.contains("network engineer"));
    }

    #[test]
    fn an_unclassified_entry_keeps_the_original_prompt() {
        let prompt = explain_entry_prompt(&entry(serde_json::json!({"facility": "local0"})));
        assert!(!prompt.contains("network device log"));
        assert!(prompt.contains("senior software engineer"));
    }

    #[test]
    fn absent_entities_are_left_out_rather_than_sent_as_empty() {
        let prompt = explain_entry_prompt(&entry(serde_json::json!({
            "event_type": "wifi_auth_failure"
        })));
        assert!(!prompt.contains("client_mac"));
        assert!(!prompt.contains("ssid"));
    }
}
