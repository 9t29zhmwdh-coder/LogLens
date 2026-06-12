use crate::models::log_entry::LogFormat;

pub fn detect_format(line: &str) -> LogFormat {
    let trimmed = line.trim();

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return LogFormat::Json;
    }

    // Nginx combined: 127.0.0.1 - frank [10/Oct/2000:13:55:36 ...] "GET / HTTP/1.1" 200 ...
    if NGINX_RE.is_match(trimmed) {
        return LogFormat::Nginx;
    }

    // Syslog: Jan  1 00:00:00 host service[pid]: msg
    if SYSLOG_RE.is_match(trimmed) {
        return LogFormat::Syslog;
    }

    // Docker JSON-File log: {"log":"...","stream":"stdout","time":"..."}
    if trimmed.starts_with("{\"log\":") {
        return LogFormat::Docker;
    }

    // Key=Value: key=val key2="val2"
    if KV_RE.is_match(trimmed) {
        return LogFormat::KeyValue;
    }

    LogFormat::Plaintext
}

use once_cell::sync::Lazy;
use regex::Regex;

static NGINX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^\S+ - \S+ \[.+\] ".+" \d{3} \d+"#).unwrap()
});

static SYSLOG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Z][a-z]{2}\s+\d+ \d{2}:\d{2}:\d{2} \S+ \S+").unwrap()
});

static KV_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\w+=(?:"[^"]*"|\S+)"#).unwrap()
});
