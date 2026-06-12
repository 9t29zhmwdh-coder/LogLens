use crate::models::log_entry::{NormalizedEntry, LogSource, LogLevel, LogFormat};
use chrono::{DateTime, Utc};

pub fn parse(line: &str, source: &LogSource) -> Option<NormalizedEntry> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let obj = v.as_object()?;

    // Docker JSON-File driver: {"log":"...","stream":"stdout","time":"..."}
    if let Some(inner) = obj.get("log").and_then(|l| l.as_str()) {
        let inner = inner.trim_end_matches('\n');
        let ts = obj.get("time")
            .and_then(|t| t.as_str())
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let mut entry = NormalizedEntry::new(
            &source.id, &source.label, ts, LogLevel::Unknown,
            inner, LogFormat::Docker,
        );
        entry.raw = line.to_string();
        return Some(entry);
    }

    // Structured JSON log
    let ts = extract_timestamp(obj).unwrap_or_else(Utc::now);
    let level = extract_level(obj);
    let message = extract_message(obj).unwrap_or_default();
    let service = obj.get("service").or(obj.get("app")).or(obj.get("logger"))
        .and_then(|v| v.as_str()).map(str::to_string);

    let mut entry = NormalizedEntry::new(
        &source.id, &source.label, ts, level, &message, LogFormat::Json,
    );
    entry.service = service;
    entry.fields = v.clone();
    entry.raw = line.to_string();

    Some(entry)
}

fn extract_timestamp(obj: &serde_json::Map<String, serde_json::Value>) -> Option<DateTime<Utc>> {
    for key in &["timestamp", "time", "@timestamp", "ts", "datetime", "date"] {
        if let Some(v) = obj.get(*key) {
            if let Some(s) = v.as_str() {
                if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }
    }
    None
}

fn extract_level(obj: &serde_json::Map<String, serde_json::Value>) -> LogLevel {
    for key in &["level", "severity", "lvl", "log_level", "loglevel"] {
        if let Some(v) = obj.get(*key) {
            if let Some(s) = v.as_str() {
                return LogLevel::from_str(s);
            }
        }
    }
    LogLevel::Unknown
}

fn extract_message(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    for key in &["message", "msg", "text", "body", "log", "event"] {
        if let Some(v) = obj.get(*key) {
            if let Some(s) = v.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}
