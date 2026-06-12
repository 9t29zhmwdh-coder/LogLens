use crate::models::log_entry::NormalizedEntry;
use crate::models::analysis::AiSummary;

pub fn export_markdown(entries: &[NormalizedEntry], summary: Option<&AiSummary>) -> String {
    let mut out = String::new();
    out.push_str("# LogLens Export\n\n");
    out.push_str(&format!("**Exported:** {}  \n", chrono::Utc::now().to_rfc3339()));
    out.push_str(&format!("**Entries:** {}\n\n", entries.len()));

    if let Some(s) = summary {
        out.push_str("## AI Summary\n\n");
        out.push_str(&format!("{}\n\n", s.overview));

        if !s.key_issues.is_empty() {
            out.push_str("### Key Issues\n\n");
            for issue in &s.key_issues {
                out.push_str(&format!("- {}\n", issue));
            }
            out.push('\n');
        }

        if !s.recommendations.is_empty() {
            out.push_str("### Recommendations\n\n");
            for rec in &s.recommendations {
                out.push_str(&format!("- {}\n", rec));
            }
            out.push('\n');
        }
    }

    out.push_str("## Log Entries\n\n");
    out.push_str("| Timestamp | Level | Service | Message |\n");
    out.push_str("|-----------|-------|---------|--------|\n");

    for entry in entries.iter().take(500) {
        let svc = entry.service.as_deref().unwrap_or("-");
        let msg = entry.message.chars().take(120).collect::<String>().replace('|', "\\|");
        out.push_str(&format!(
            "| {} | {:?} | {} | {} |\n",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.level, svc, msg
        ));
    }

    if entries.len() > 500 {
        out.push_str(&format!("\n_... and {} more entries_\n", entries.len() - 500));
    }

    out
}
