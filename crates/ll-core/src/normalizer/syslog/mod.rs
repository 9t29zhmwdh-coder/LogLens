//! Syslog wire-format parsing, RFC 5424 and RFC 3164.
//!
//! Network gear is the last place where both formats are still in daily use:
//! a UniFi gateway emits 3164, a modern appliance emits 5424, and a site with
//! both sends them to the same collector on the same port. Which one a line is
//! can only be decided per line, never per source.

pub mod rfc3164;
pub mod rfc5424;

use crate::models::log_entry::{LogFormat, LogLevel, LogSource, NormalizedEntry};
use crate::models::network_event::NetworkEvent;
use crate::plugin::network::ClassifierRegistry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Syslog facility, the subsystem that produced the message (RFC 5424 §6.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facility {
    Kernel,
    User,
    Mail,
    Daemon,
    Auth,
    Syslog,
    Lpr,
    News,
    Uucp,
    Cron,
    AuthPriv,
    Ftp,
    Ntp,
    LogAudit,
    LogAlert,
    ClockDaemon,
    Local(u8),
}

impl Facility {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Kernel,
            1 => Self::User,
            2 => Self::Mail,
            3 => Self::Daemon,
            4 => Self::Auth,
            5 => Self::Syslog,
            6 => Self::Lpr,
            7 => Self::News,
            8 => Self::Uucp,
            9 => Self::Cron,
            10 => Self::AuthPriv,
            11 => Self::Ftp,
            12 => Self::Ntp,
            13 => Self::LogAudit,
            14 => Self::LogAlert,
            15 => Self::ClockDaemon,
            n => Self::Local(n.saturating_sub(16)),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Self::Kernel => "kernel".into(),
            Self::User => "user".into(),
            Self::Mail => "mail".into(),
            Self::Daemon => "daemon".into(),
            Self::Auth => "auth".into(),
            Self::Syslog => "syslog".into(),
            Self::Lpr => "lpr".into(),
            Self::News => "news".into(),
            Self::Uucp => "uucp".into(),
            Self::Cron => "cron".into(),
            Self::AuthPriv => "authpriv".into(),
            Self::Ftp => "ftp".into(),
            Self::Ntp => "ntp".into(),
            Self::LogAudit => "logaudit".into(),
            Self::LogAlert => "logalert".into(),
            Self::ClockDaemon => "clockdaemon".into(),
            Self::Local(n) => format!("local{n}"),
        }
    }
}

/// Syslog severity, 0 (emergency) through 7 (debug).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Informational,
    Debug,
}

impl Severity {
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Emergency,
            1 => Self::Alert,
            2 => Self::Critical,
            3 => Self::Error,
            4 => Self::Warning,
            5 => Self::Notice,
            6 => Self::Informational,
            _ => Self::Debug,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Emergency => "emergency",
            Self::Alert => "alert",
            Self::Critical => "critical",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Notice => "notice",
            Self::Informational => "informational",
            Self::Debug => "debug",
        }
    }
}

/// Splits a `<PRI>` value into its facility and severity halves.
/// Values above 191 are out of range per RFC 5424 and are rejected.
pub fn decode_priority(pri: u16) -> Option<(Facility, Severity)> {
    if pri > 191 {
        return None;
    }
    let pri = pri as u8;
    Some((Facility::from_code(pri >> 3), Severity::from_code(pri & 0b111)))
}

/// A parsed syslog line, before any vendor-specific interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct SyslogMessage {
    pub facility: Facility,
    pub severity: Severity,
    pub timestamp: Option<DateTime<Utc>>,
    pub hostname: Option<String>,
    pub app_name: Option<String>,
    pub proc_id: Option<String>,
    pub msg_id: Option<String>,
    /// RFC 5424 structured data, flattened to `sd_id@enterprise.param` keys.
    pub structured_data: Vec<(String, String)>,
    pub message: String,
}

/// Parses a syslog line, trying RFC 5424 first because its version marker
/// makes it unambiguous, then falling back to the looser RFC 3164 shape.
pub fn parse(line: &str) -> Option<SyslogMessage> {
    rfc5424::parse(line).or_else(|| rfc3164::parse(line))
}

impl Severity {
    /// Syslog has eight levels, the rest of the application has seven. The
    /// mapping keeps the operator-facing meaning: notice is not a warning,
    /// and alert and emergency both mean the same thing to a human at 3am.
    pub fn to_log_level(self) -> LogLevel {
        match self {
            Self::Emergency | Self::Alert => LogLevel::Fatal,
            Self::Critical | Self::Error => LogLevel::Error,
            Self::Warning => LogLevel::Warn,
            Self::Notice | Self::Informational => LogLevel::Info,
            Self::Debug => LogLevel::Debug,
        }
    }
}

/// Parses a syslog line and, where a classifier recognizes it, attaches the
/// network event to the entry's `fields`.
///
/// Returns `None` only when the line is not syslog at all, so an
/// unrecognized-but-well-formed line still reaches search and clustering.
pub fn normalize_line(
    line: &str,
    source: &LogSource,
    classifiers: &ClassifierRegistry,
) -> Option<NormalizedEntry> {
    let msg = parse(line)?;
    let event = classifiers.classify(&msg);

    let mut fields = serde_json::Map::new();
    fields.insert("facility".into(), msg.facility.as_str().into());
    fields.insert("severity".into(), msg.severity.as_str().into());
    if let Some(host) = &msg.hostname {
        fields.insert("hostname".into(), host.clone().into());
    }
    for (key, value) in &msg.structured_data {
        fields.insert(format!("sd.{key}"), value.clone().into());
    }
    if let Some(event) = &event {
        insert_network_event(&mut fields, event);
    }

    Some(NormalizedEntry {
        id: uuid::Uuid::new_v4().to_string(),
        source_id: source.id.clone(),
        source_label: source.label.clone(),
        timestamp: msg.timestamp.unwrap_or_else(Utc::now),
        level: msg.severity.to_log_level(),
        service: msg.app_name.clone(),
        message: msg.message.clone(),
        stacktrace: None,
        fields: serde_json::Value::Object(fields),
        raw: line.to_string(),
        format: LogFormat::Syslog,
        fingerprint: String::new(),
        cluster_id: None,
        ingested_at: Utc::now(),
    })
}

/// Flattens the network event into the entry's fields so that the existing
/// full-text index makes MACs, addresses and event types searchable without
/// any schema change.
fn insert_network_event(fields: &mut serde_json::Map<String, serde_json::Value>, event: &NetworkEvent) {
    fields.insert("vendor".into(), event.vendor.as_str().into());
    fields.insert("event_type".into(), event.event_type.as_str().into());
    if let Some(key) = &event.vendor_event_key {
        fields.insert("vendor_event_key".into(), key.clone().into());
    }
    let entities = &event.entities;
    let mut put = |key: &str, value: Option<String>| {
        if let Some(value) = value {
            fields.insert(key.to_string(), value.into());
        }
    };
    put("client_mac", entities.client_mac.map(|m| m.to_string()));
    put("device_mac", entities.device_mac.map(|m| m.to_string()));
    put("src_ip", entities.src_ip.map(|ip| ip.to_string()));
    put("dst_ip", entities.dst_ip.map(|ip| ip.to_string()));
    put("src_port", entities.src_port.map(|p| p.to_string()));
    put("dst_port", entities.dst_port.map(|p| p.to_string()));
    put("protocol", entities.protocol.map(|p| p.to_string()));
    put("ssid", entities.ssid.clone());
    put("vlan", entities.vlan.map(|v| v.to_string()));
    put("interface", entities.interface.clone());
    put("rule_id", entities.rule_id.clone());
    put("radio_band", entities.radio_band.clone());
    put("reason", entities.reason.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_splits_into_facility_and_severity() {
        // 134 = local0 (16) * 8 + informational (6)
        let (facility, severity) = decode_priority(134).unwrap();
        assert_eq!(facility, Facility::Local(0));
        assert_eq!(severity, Severity::Informational);
    }

    #[test]
    fn kernel_emergency_is_priority_zero() {
        let (facility, severity) = decode_priority(0).unwrap();
        assert_eq!(facility, Facility::Kernel);
        assert_eq!(severity, Severity::Emergency);
    }

    #[test]
    fn out_of_range_priority_is_rejected() {
        assert!(decode_priority(192).is_none());
        assert!(decode_priority(1000).is_none());
        assert!(decode_priority(191).is_some());
    }

    #[test]
    fn severity_orders_most_urgent_first() {
        assert!(Severity::Emergency < Severity::Debug);
        assert!(Severity::Error < Severity::Warning);
    }
}
