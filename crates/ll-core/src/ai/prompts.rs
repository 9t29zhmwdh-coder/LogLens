use crate::models::log_entry::NormalizedEntry;
use crate::models::cluster::LogCluster;

pub fn explain_entry_prompt(entry: &NormalizedEntry) -> String {
    let stacktrace = entry.stacktrace.as_ref()
        .map(|lines| format!("\nStacktrace:\n{}", lines.join("\n")))
        .unwrap_or_default();

    format!(
        r#"You are a senior software engineer analyzing a log entry.

Log entry:
- Timestamp: {}
- Level: {:?}
- Service: {}
- Message: {}{}
- Raw: {}

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
