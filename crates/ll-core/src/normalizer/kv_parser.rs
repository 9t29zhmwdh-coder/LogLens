use crate::models::log_entry::{NormalizedEntry, LogSource, LogLevel, LogFormat};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;

static KV_PAIR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)=(?:"([^"]*)"|(\S+))"#).unwrap()
});

pub fn parse(line: &str, source: &LogSource) -> Option<NormalizedEntry> {
    if line.trim().is_empty() {
        return None;
    }

    let mut map = serde_json::Map::new();
    for cap in KV_PAIR.captures_iter(line) {
        let key = cap[1].to_string();
        let val = cap.get(2).or(cap.get(3)).map(|m| m.as_str()).unwrap_or("");
        map.insert(key, serde_json::Value::String(val.to_string()));
    }

    let ts = map.get("time").or(map.get("ts")).or(map.get("timestamp"))
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    let level = map.get("level").or(map.get("lvl"))
        .and_then(|v| v.as_str())
        .map(LogLevel::from_str)
        .unwrap_or(LogLevel::Unknown);

    let message = map.get("msg").or(map.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or(line)
        .to_string();

    let service = map.get("service").or(map.get("app"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let mut entry = NormalizedEntry::new(
        &source.id, &source.label, ts, level, &message, LogFormat::KeyValue,
    );
    entry.service = service;
    entry.fields = serde_json::Value::Object(map);
    entry.raw = line.to_string();
    Some(entry)
}
