use crate::models::log_entry::{NormalizedEntry, LogSource, LogLevel, LogFormat};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;

// Matches common datetime patterns at line start
static TS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)^
        (?:
            (\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?) |  # ISO
            ([A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2})                                       # syslog
        )\s*"
    ).unwrap()
});

static LEVEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(TRACE|DEBUG|INFO|WARN(?:ING)?|ERROR|ERR|FATAL|CRITICAL|CRIT)\b").unwrap()
});

static SERVICE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[([A-Za-z][\w\-\.]{1,40})\]").unwrap()
});

pub fn parse(line: &str, source: &LogSource) -> Option<NormalizedEntry> {
    if line.trim().is_empty() {
        return None;
    }

    let mut rest = line;
    let ts = if let Some(m) = TS_RE.find(rest) {
        let ts_str = m.as_str().trim();
        rest = &rest[m.end()..];
        parse_ts(ts_str)
    } else {
        Utc::now()
    };

    let level = LEVEL_RE.find(rest)
        .map(|m| LogLevel::from_str(m.as_str()))
        .unwrap_or(LogLevel::Unknown);

    let service = SERVICE_RE.captures(rest)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());

    // Strip leading level / brackets to get the message
    let message = LEVEL_RE.replace(rest, "")
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '.')
        .to_string();

    let mut entry = NormalizedEntry::new(
        &source.id, &source.label, ts, level,
        if message.is_empty() { rest.trim() } else { &message },
        LogFormat::Plaintext,
    );
    entry.service = service;
    entry.raw = line.to_string();
    Some(entry)
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
