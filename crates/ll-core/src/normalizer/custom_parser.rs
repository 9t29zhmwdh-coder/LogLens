//! User-defined parsers via a regex template: an org has a log format none
//! of the built-in parsers (JSON, key-value, nginx, syslog, plaintext)
//! recognizes, and rather than shipping a new built-in for every format,
//! lets the user describe it with named capture groups.
//!
//! A template's `line_regex` uses the named groups `timestamp`, `level`,
//! `service`, and `message` (all optional; missing ones fall back to
//! `Utc::now()`, `Unknown`, `None`, and the whole line respectively). A
//! `LogSource` opts into a template by setting `parser_hint` to the
//! template's `id`.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::models::log_entry::{LogFormat, LogLevel, LogSource, NormalizedEntry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomParserTemplate {
    pub id: String,
    pub name: String,
    pub line_regex: String,
    /// A chrono strftime pattern for the `timestamp` group. `None` tries
    /// RFC 3339 first and falls back to the current time.
    #[serde(default)]
    pub timestamp_format: Option<String>,
}

pub struct CompiledCustomParser {
    template: CustomParserTemplate,
    regex: Regex,
}

/// A set of user templates, each compiled once up front rather than per
/// line: `Regex::new` is not cheap enough to redo per log line.
#[derive(Default)]
pub struct CustomParserSet(HashMap<String, CompiledCustomParser>);

impl CustomParserSet {
    /// Templates with an invalid regex are skipped (with a warning), not
    /// fatal: one bad template must not take down parsing for every source.
    pub fn compile(templates: &[CustomParserTemplate]) -> Self {
        let mut set = HashMap::new();
        for template in templates {
            match Regex::new(&template.line_regex) {
                Ok(regex) => {
                    set.insert(
                        template.id.clone(),
                        CompiledCustomParser { template: template.clone(), regex },
                    );
                }
                Err(e) => {
                    tracing::warn!("custom parser '{}' has an invalid regex, skipping: {}", template.id, e);
                }
            }
        }
        Self(set)
    }

    pub fn get(&self, id: &str) -> Option<&CompiledCustomParser> {
        self.0.get(id)
    }
}

/// Returns `None` if the line does not match the template's regex at all;
/// callers should fall back to a built-in parser rather than drop the line.
pub fn parse(parser: &CompiledCustomParser, line: &str, source: &LogSource) -> Option<NormalizedEntry> {
    if line.trim().is_empty() {
        return None;
    }

    let caps = parser.regex.captures(line)?;

    let message = caps.name("message").map(|m| m.as_str().to_string())
        .unwrap_or_else(|| line.to_string());

    let level = caps.name("level")
        .map(|m| LogLevel::from_str(m.as_str()))
        .unwrap_or(LogLevel::Unknown);

    let service = caps.name("service").map(|m| m.as_str().to_string());

    let timestamp = caps.name("timestamp")
        .and_then(|m| parse_timestamp(m.as_str(), parser.template.timestamp_format.as_deref()))
        .unwrap_or_else(Utc::now);

    let mut entry = NormalizedEntry::new(&source.id, &source.label, timestamp, level, message, LogFormat::Unknown);
    entry.service = service;
    entry.raw = line.to_string();
    Some(entry)
}

fn parse_timestamp(raw: &str, format: Option<&str>) -> Option<DateTime<Utc>> {
    if let Some(fmt) = format {
        if let Ok(naive) = NaiveDateTime::parse_from_str(raw, fmt) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }
    DateTime::parse_from_rfc3339(raw).ok().map(|t| t.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> LogSource {
        LogSource::new("test", crate::models::log_entry::LogSourceKind::Stdin)
    }

    fn template() -> CustomParserTemplate {
        CustomParserTemplate {
            id: "acme".to_string(),
            name: "Acme Format".to_string(),
            line_regex: r"^(?P<timestamp>\S+) \[(?P<level>\w+)\] (?P<service>[\w-]+): (?P<message>.*)$".to_string(),
            timestamp_format: None,
        }
    }

    #[test]
    fn parses_all_named_groups() {
        let set = CustomParserSet::compile(&[template()]);
        let parser = set.get("acme").unwrap();
        let entry = parse(parser, "2026-07-13T10:00:00Z [ERROR] billing-svc: charge declined", &source()).unwrap();
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.service, Some("billing-svc".to_string()));
        assert_eq!(entry.message, "charge declined");
        assert_eq!(entry.timestamp.to_rfc3339(), "2026-07-13T10:00:00+00:00");
    }

    #[test]
    fn non_matching_line_returns_none() {
        let set = CustomParserSet::compile(&[template()]);
        let parser = set.get("acme").unwrap();
        assert!(parse(parser, "this does not match at all", &source()).is_none());
    }

    #[test]
    fn missing_optional_groups_use_defaults() {
        let minimal = CustomParserTemplate {
            id: "minimal".to_string(),
            name: "Minimal".to_string(),
            line_regex: r"^(?P<message>.*)$".to_string(),
            timestamp_format: None,
        };
        let set = CustomParserSet::compile(&[minimal]);
        let parser = set.get("minimal").unwrap();
        let entry = parse(parser, "just a plain line", &source()).unwrap();
        assert_eq!(entry.level, LogLevel::Unknown);
        assert_eq!(entry.service, None);
        assert_eq!(entry.message, "just a plain line");
    }

    #[test]
    fn invalid_regex_is_skipped_not_fatal() {
        let bad = CustomParserTemplate {
            id: "bad".to_string(),
            name: "Bad".to_string(),
            line_regex: "(unclosed".to_string(),
            timestamp_format: None,
        };
        let set = CustomParserSet::compile(&[bad, template()]);
        assert!(set.get("bad").is_none());
        assert!(set.get("acme").is_some());
    }

    #[test]
    fn custom_timestamp_format_is_respected() {
        let template = CustomParserTemplate {
            id: "custom-ts".to_string(),
            name: "Custom TS".to_string(),
            line_regex: r"^(?P<timestamp>\S+ \S+) (?P<message>.*)$".to_string(),
            timestamp_format: Some("%Y/%m/%d %H:%M:%S".to_string()),
        };
        let set = CustomParserSet::compile(&[template]);
        let parser = set.get("custom-ts").unwrap();
        let entry = parse(parser, "2026/07/13 10:00:00 hello", &source()).unwrap();
        assert_eq!(entry.timestamp.to_rfc3339(), "2026-07-13T10:00:00+00:00");
    }
}
