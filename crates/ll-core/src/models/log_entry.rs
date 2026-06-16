use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().trim_matches(|c: char| !c.is_alphanumeric()) {
            "trace" => Self::Trace,
            "debug" | "dbg" | "d" => Self::Debug,
            "info" | "information" | "i" => Self::Info,
            "warn" | "warning" | "w" => Self::Warn,
            "error" | "err" | "e" => Self::Error,
            "fatal" | "critical" | "crit" | "f" => Self::Fatal,
            _ => Self::Unknown,
        }
    }

    pub fn score(&self) -> u8 {
        match self {
            Self::Trace => 0, Self::Debug => 1, Self::Info => 2,
            Self::Warn => 3, Self::Error => 4, Self::Fatal => 5, Self::Unknown => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogSourceKind {
    File { path: String },
    Directory { path: String, pattern: Option<String> },
    DockerContainer { container_id: String, name: String },
    DockerService { service_name: String },
    Stdin,
    SystemMacos,
    Journald,
    WindowsEventLog { channel: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSource {
    pub id: String,
    pub label: String,
    pub kind: LogSourceKind,
    pub parser_hint: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl LogSource {
    pub fn new(label: impl Into<String>, kind: LogSourceKind) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.into(),
            kind,
            parser_hint: None,
            enabled: true,
            created_at: Utc::now(),
        }
    }
}

/// Raw entry before normalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLogEntry {
    pub id: String,
    pub source_id: String,
    pub raw_lines: Vec<String>,
    pub collected_at: DateTime<Utc>,
}

/// Normalized entry — canonical representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedEntry {
    pub id: String,
    pub source_id: String,
    pub source_label: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub service: Option<String>,
    pub message: String,
    pub stacktrace: Option<Vec<String>>,
    pub fields: serde_json::Value,
    pub raw: String,
    pub format: LogFormat,
    pub fingerprint: String,
    pub cluster_id: Option<String>,
    pub ingested_at: DateTime<Utc>,
}

impl NormalizedEntry {
    pub fn new(
        source_id: impl Into<String>,
        source_label: impl Into<String>,
        timestamp: DateTime<Utc>,
        level: LogLevel,
        message: impl Into<String>,
        format: LogFormat,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.into(),
            source_label: source_label.into(),
            timestamp,
            level,
            service: None,
            message: message.into(),
            stacktrace: None,
            fields: serde_json::Value::Object(Default::default()),
            raw: String::new(),
            format,
            fingerprint: String::new(),
            cluster_id: None,
            ingested_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    Json,
    Plaintext,
    KeyValue,
    Nginx,
    Docker,
    Syslog,
    WindowsEvent,
    Unknown,
}
