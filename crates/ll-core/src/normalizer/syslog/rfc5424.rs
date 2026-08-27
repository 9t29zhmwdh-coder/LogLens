//! RFC 5424 syslog parsing.
//!
//! Layout: `<PRI>VERSION TIMESTAMP HOSTNAME APP-NAME PROCID MSGID SD [MSG]`
//! with `-` as the nil value for any header field.

use super::{decode_priority, SyslogMessage};
use chrono::{DateTime, Utc};

/// Parses a line as RFC 5424. Returns `None` if the line does not carry the
/// version marker, which is what separates 5424 from the older 3164 shape.
pub fn parse(line: &str) -> Option<SyslogMessage> {
    let (pri, rest) = split_priority(line)?;
    let (facility, severity) = decode_priority(pri)?;

    // Only version 1 exists; anything else is not a 5424 line we understand.
    let rest = rest.strip_prefix("1 ")?;

    let mut fields = rest.splitn(6, ' ');
    let timestamp = parse_timestamp(fields.next()?);
    let hostname = nilable(fields.next()?);
    let app_name = nilable(fields.next()?);
    let proc_id = nilable(fields.next()?);
    let msg_id = nilable(fields.next()?);

    let tail = fields.next().unwrap_or("");
    let (structured_data, message) = split_structured_data(tail);

    Some(SyslogMessage {
        facility,
        severity,
        timestamp,
        hostname,
        app_name,
        proc_id,
        msg_id,
        structured_data,
        message,
    })
}

/// Reads the leading `<N>` and returns the numeric priority with the remainder.
pub(super) fn split_priority(line: &str) -> Option<(u16, &str)> {
    let rest = line.strip_prefix('<')?;
    let end = rest.find('>')?;
    let pri = rest[..end].parse::<u16>().ok()?;
    Some((pri, &rest[end + 1..]))
}

/// RFC 5424 spells an absent header field as a single hyphen.
fn nilable(field: &str) -> Option<String> {
    match field {
        "-" | "" => None,
        other => Some(other.to_string()),
    }
}

fn parse_timestamp(field: &str) -> Option<DateTime<Utc>> {
    if field == "-" {
        return None;
    }
    DateTime::parse_from_rfc3339(field).ok().map(|dt| dt.with_timezone(&Utc))
}

/// Separates the structured-data block from the free-text message.
///
/// The block is either a lone `-` or one or more `[...]` elements. Brackets
/// inside quoted values are escaped as `\]`, so scanning has to respect
/// quoting rather than counting brackets naively.
fn split_structured_data(tail: &str) -> (Vec<(String, String)>, String) {
    if let Some(rest) = tail.strip_prefix("- ") {
        return (Vec::new(), strip_bom(rest).to_string());
    }
    if tail == "-" {
        return (Vec::new(), String::new());
    }
    if !tail.starts_with('[') {
        return (Vec::new(), strip_bom(tail).to_string());
    }

    let end = find_sd_end(tail);
    let (sd_block, msg) = tail.split_at(end);
    let message = strip_bom(msg.strip_prefix(' ').unwrap_or(msg)).to_string();
    (parse_sd_elements(sd_block), message)
}

/// Returns the byte offset just past the final `]` of the structured-data
/// block, treating `\]` inside a quoted value as literal.
fn find_sd_end(tail: &str) -> usize {
    let bytes = tail.as_bytes();
    let (mut i, mut in_quotes, mut depth) = (0usize, false, 0i32);
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_quotes => i += 1,
            b'"' => in_quotes = !in_quotes,
            b'[' if !in_quotes => depth += 1,
            b']' if !in_quotes => {
                depth -= 1;
                if depth == 0 && bytes.get(i + 1) != Some(&b'[') {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    tail.len()
}

/// Flattens `[id param="v"]` elements into `id.param` / `v` pairs.
fn parse_sd_elements(block: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for element in split_elements(block) {
        let mut parts = element.splitn(2, ' ');
        let Some(sd_id) = parts.next() else { continue };
        let Some(params) = parts.next() else { continue };
        for (key, value) in parse_params(params) {
            out.push((format!("{sd_id}.{key}"), value));
        }
    }
    out
}

fn split_elements(block: &str) -> Vec<String> {
    let bytes = block.as_bytes();
    let (mut out, mut start, mut in_quotes) = (Vec::new(), None, false);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_quotes => i += 1,
            b'"' => in_quotes = !in_quotes,
            b'[' if !in_quotes => start = Some(i + 1),
            b']' if !in_quotes => {
                if let Some(s) = start.take() {
                    out.push(block[s..i].to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// Parses `key="value"` pairs, unescaping `\"`, `\\` and `\]`.
fn parse_params(params: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut chars = params.chars().peekable();
    let mut key = String::new();
    while let Some(c) = chars.next() {
        match c {
            '=' => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    out.push((key.trim().to_string(), read_quoted(&mut chars)));
                }
                key.clear();
            }
            ' ' if key.is_empty() => {}
            _ => key.push(c),
        }
    }
    out
}

fn read_quoted(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut value = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    value.push(escaped);
                }
            }
            '"' => break,
            _ => value.push(c),
        }
    }
    value
}

/// RFC 5424 marks a UTF-8 message with a leading byte order mark; it is a
/// transport detail, not part of the message an operator wants to read.
fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::syslog::{Facility, Severity};

    // The four examples in RFC 5424 section 6.5, verbatim.

    #[test]
    fn rfc_example_1_without_structured_data() {
        let line = "<34>1 2003-10-11T22:14:15.003Z mymachine.example.com su - ID47 - BOM'su root' failed for lonvick on /dev/pts/8";
        let m = parse(line).unwrap();
        assert_eq!(m.facility, Facility::Auth);
        assert_eq!(m.severity, Severity::Critical);
        assert_eq!(m.hostname.as_deref(), Some("mymachine.example.com"));
        assert_eq!(m.app_name.as_deref(), Some("su"));
        assert_eq!(m.proc_id, None);
        assert_eq!(m.msg_id.as_deref(), Some("ID47"));
        assert!(m.structured_data.is_empty());
        assert!(m.message.starts_with("BOM'su root' failed"));
    }

    #[test]
    fn rfc_example_2_with_procid_and_no_message() {
        let line = "<165>1 2003-08-24T05:14:15.000003-07:00 192.0.2.1 myproc 8710 - - %% It's time to make the do-nuts.";
        let m = parse(line).unwrap();
        assert_eq!(m.proc_id.as_deref(), Some("8710"));
        assert_eq!(m.message, "%% It's time to make the do-nuts.");
        assert_eq!(m.timestamp.unwrap().to_rfc3339(), "2003-08-24T12:14:15.000003+00:00");
    }

    #[test]
    fn rfc_example_3_with_one_structured_data_element() {
        let line = r#"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"] BOMAn application event log entry..."#;
        let m = parse(line).unwrap();
        assert_eq!(m.structured_data.len(), 3);
        assert_eq!(m.structured_data[0], ("exampleSDID@32473.iut".into(), "3".into()));
        assert_eq!(m.structured_data[1], ("exampleSDID@32473.eventSource".into(), "Application".into()));
        // "BOM" here is the RFC document's placeholder spelling, not the byte itself.
        assert_eq!(m.message, "BOMAn application event log entry...");
    }

    #[test]
    fn rfc_example_4_with_two_structured_data_elements_and_no_message() {
        let line = r#"<165>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3"][examplePriority@32473 class="high"]"#;
        let m = parse(line).unwrap();
        assert_eq!(m.structured_data.len(), 2);
        assert_eq!(m.structured_data[1], ("examplePriority@32473.class".into(), "high".into()));
        assert_eq!(m.message, "");
    }

    #[test]
    fn escaped_bracket_inside_a_value_does_not_end_the_block() {
        let line = r#"<134>1 2026-08-27T10:00:00Z fw filterlog - - [meta@1 rule="deny \] all"] blocked"#;
        let m = parse(line).unwrap();
        assert_eq!(m.structured_data[0], ("meta@1.rule".into(), "deny ] all".into()));
        assert_eq!(m.message, "blocked");
    }

    #[test]
    fn a_bom_prefixed_message_is_delivered_without_it() {
        let line = "<134>1 2026-08-27T10:00:00Z host app - - - \u{feff}real message";
        assert_eq!(parse(line).unwrap().message, "real message");
    }

    #[test]
    fn rfc3164_shaped_lines_are_refused() {
        // No version marker, so this must fall through to the 3164 parser.
        assert!(parse("<34>Oct 11 22:14:15 mymachine su: failed").is_none());
    }

    #[test]
    fn malformed_priority_is_refused() {
        assert!(parse("no priority here").is_none());
        assert!(parse("<999>1 2026-08-27T10:00:00Z h a - - - m").is_none());
        assert!(parse("<abc>1 2026-08-27T10:00:00Z h a - - - m").is_none());
    }

    #[test]
    fn an_unparseable_timestamp_does_not_lose_the_message() {
        let m = parse("<134>1 not-a-time host app - - - still readable").unwrap();
        assert!(m.timestamp.is_none());
        assert_eq!(m.message, "still readable");
    }
}
