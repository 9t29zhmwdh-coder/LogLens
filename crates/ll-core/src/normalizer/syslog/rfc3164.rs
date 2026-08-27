//! RFC 3164 syslog parsing, the "BSD" format still emitted by most network
//! appliances.
//!
//! Layout: `<PRI>Mmm dd hh:mm:ss HOSTNAME TAG[PID]: MSG`. Every part after
//! the priority is optional in practice: Mikrotik omits the hostname, some
//! firmware omits the timestamp, and the tag is free-form. The parser
//! therefore recovers what it can rather than rejecting the line.

use super::{decode_priority, rfc5424::split_priority, SyslogMessage};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// Parses a line as RFC 3164. Requires only a valid `<PRI>`; everything after
/// it is best-effort, so a line whose shape is unusual still yields a message.
pub fn parse(line: &str) -> Option<SyslogMessage> {
    parse_with_reference(line, Utc::now())
}

/// Same as [`parse`], with the "now" used to infer the missing year injected
/// so the year-rollover behaviour can be tested deterministically.
pub fn parse_with_reference(line: &str, reference: DateTime<Utc>) -> Option<SyslogMessage> {
    let (pri, rest) = split_priority(line)?;
    let (facility, severity) = decode_priority(pri)?;

    let (timestamp, rest) = match take_timestamp(rest, reference) {
        Some((ts, rest)) => (Some(ts), rest),
        None => (None, rest),
    };
    let (hostname, rest) = take_hostname(rest);
    let (app_name, proc_id, message) = take_tag(rest);

    Some(SyslogMessage {
        facility,
        severity,
        timestamp,
        hostname,
        app_name,
        proc_id,
        msg_id: None,
        structured_data: Vec::new(),
        message,
    })
}

/// Reads `Mmm dd hh:mm:ss`, where the day may be space-padded (`Oct  1`).
fn take_timestamp(rest: &str, reference: DateTime<Utc>) -> Option<(DateTime<Utc>, &str)> {
    // Slicing by byte index is only safe once the prefix is known to be
    // ASCII; a line starting with a multi-byte character would otherwise
    // panic on a character boundary.
    if !rest.is_char_boundary(15) || !rest.is_ascii() {
        return parse_ascii_timestamp(rest.get(..15)?, reference).map(|ts| (ts, rest.get(16..).unwrap_or("")));
    }
    let month = month_from_name(rest.get(..3)?)?;
    let day: u32 = rest.get(4..6)?.trim().parse().ok()?;
    let (hour, minute, second) = parse_clock(rest.get(7..15)?)?;

    let year = infer_year(month, day, reference);
    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let naive = date.and_hms_opt(hour, minute, second)?;
    let stamp = Utc.from_utc_datetime(&naive);
    Some((stamp, rest.get(16..).unwrap_or("")))
}

fn month_from_name(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    MONTHS.iter().position(|m| *m == lower).map(|i| i as u32 + 1)
}

fn parse_clock(time: &str) -> Option<(u32, u32, u32)> {
    let mut parts = time.split(':');
    let hour = parts.next()?.parse().ok()?;
    let minute = parts.next()?.parse().ok()?;
    let second = parts.next()?.parse().ok()?;
    if hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some((hour, minute, second))
}

/// RFC 3164 carries no year. A stamp that would land in the future is
/// therefore from last year, which is what happens to every line still in
/// flight across New Year's Eve. One day of slack absorbs clock skew between
/// the sending device and the collector.
fn infer_year(month: u32, day: u32, reference: DateTime<Utc>) -> i32 {
    let year = reference.year();
    let Some(candidate) = NaiveDate::from_ymd_opt(year, month, day) else {
        return year;
    };
    let stamp = Utc.from_utc_datetime(&candidate.and_hms_opt(0, 0, 0).unwrap());
    if stamp > reference + chrono::Duration::days(1) {
        year - 1
    } else {
        year
    }
}

/// The hostname is the next whitespace-delimited token, unless that token
/// already looks like a tag (`name:` or `name[pid]:`), which is how
/// hostname-less firmware presents itself.
fn take_hostname(rest: &str) -> (Option<String>, &str) {
    let rest = rest.trim_start();
    let Some(end) = rest.find(' ') else {
        return (None, rest);
    };
    let candidate = &rest[..end];
    if candidate.ends_with(':') || candidate.contains('[') {
        return (None, rest);
    }
    (Some(candidate.to_string()), &rest[end + 1..])
}

/// Splits `tag[pid]: message` into its three parts. A line without the
/// `tag:` convention keeps its full text as the message.
fn take_tag(rest: &str) -> (Option<String>, Option<String>, String) {
    let rest = rest.trim_start();
    // The delimiter is a colon followed by a space. Matching a bare colon
    // would cut a leading MAC address ("aa:bb:...") in half, which is exactly
    // how RouterOS opens its wireless lines.
    let Some(colon) = rest.find(": ") else {
        return (None, None, rest.to_string());
    };
    let head = &rest[..colon];
    // A colon far into the line is punctuation in prose, not a tag delimiter.
    if head.len() > 48 || head.contains(' ') {
        return (None, None, rest.to_string());
    }
    let message = rest[colon + 1..].trim_start().to_string();
    match head.split_once('[') {
        Some((tag, pid)) => (
            Some(tag.to_string()),
            Some(pid.trim_end_matches(']').to_string()),
            message,
        ),
        None => (Some(head.to_string()), None, message),
    }
}

/// Parses the fixed-width `Mmm dd hh:mm:ss` prefix of a line that also
/// contains non-ASCII text further along.
fn parse_ascii_timestamp(head: &str, reference: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !head.is_ascii() {
        return None;
    }
    let month = month_from_name(head.get(..3)?)?;
    let day: u32 = head.get(4..6)?.trim().parse().ok()?;
    let (hour, minute, second) = parse_clock(head.get(7..15)?)?;
    let year = infer_year(month, day, reference);
    NaiveDate::from_ymd_opt(year, month, day)?
        .and_hms_opt(hour, minute, second)
        .map(|naive| Utc.from_utc_datetime(&naive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::syslog::{Facility, Severity};

    fn reference() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 27, 12, 0, 0).unwrap()
    }

    #[test]
    fn rfc_example_parses_into_all_parts() {
        let line = "<34>Oct 11 22:14:15 mymachine su: 'su root' failed for lonvick on /dev/pts/8";
        let m = parse_with_reference(line, reference()).unwrap();
        assert_eq!(m.facility, Facility::Auth);
        assert_eq!(m.severity, Severity::Critical);
        assert_eq!(m.hostname.as_deref(), Some("mymachine"));
        assert_eq!(m.app_name.as_deref(), Some("su"));
        assert_eq!(m.message, "'su root' failed for lonvick on /dev/pts/8");
    }

    #[test]
    fn process_id_in_brackets_is_split_off() {
        let line = "<134>Aug 27 10:15:00 gateway dnsmasq[1234]: DHCPACK(br0) 192.168.1.50";
        let m = parse_with_reference(line, reference()).unwrap();
        assert_eq!(m.app_name.as_deref(), Some("dnsmasq"));
        assert_eq!(m.proc_id.as_deref(), Some("1234"));
        assert_eq!(m.message, "DHCPACK(br0) 192.168.1.50");
    }

    #[test]
    fn space_padded_single_digit_day_is_read() {
        let m = parse_with_reference("<134>Aug  3 10:15:00 host app: text", reference()).unwrap();
        assert_eq!(m.timestamp.unwrap().day(), 3);
        assert_eq!(m.message, "text");
    }

    #[test]
    fn a_december_stamp_seen_in_january_belongs_to_last_year() {
        let new_year = Utc.with_ymd_and_hms(2027, 1, 2, 3, 0, 0).unwrap();
        let m = parse_with_reference("<134>Dec 31 23:59:00 host app: crossing over", new_year).unwrap();
        assert_eq!(m.timestamp.unwrap().year(), 2026);
    }

    #[test]
    fn a_stamp_one_hour_ahead_stays_in_the_current_year() {
        // Devices whose clock runs slightly fast must not be pushed back a year.
        let m = parse_with_reference("<134>Aug 27 13:00:00 host app: slight skew", reference()).unwrap();
        assert_eq!(m.timestamp.unwrap().year(), 2026);
    }

    #[test]
    fn a_line_without_hostname_still_finds_its_tag() {
        // Mikrotik sends no hostname field.
        let m = parse_with_reference("<134>Aug 27 10:15:00 firewall: input: in:ether1 out:(none)", reference()).unwrap();
        assert_eq!(m.hostname, None);
        assert_eq!(m.app_name.as_deref(), Some("firewall"));
        assert!(m.message.starts_with("input:"));
    }

    #[test]
    fn a_colon_inside_prose_is_not_mistaken_for_a_tag() {
        let m = parse_with_reference("<134>Aug 27 10:15:00 host the link went down: check the cable", reference()).unwrap();
        assert_eq!(m.app_name, None);
        assert_eq!(m.message, "the link went down: check the cable");
    }

    #[test]
    fn a_line_without_timestamp_keeps_its_message() {
        let m = parse_with_reference("<134>some daemon shouted", reference()).unwrap();
        assert!(m.timestamp.is_none());
        assert!(m.message.contains("shouted"));
    }

    #[test]
    fn non_ascii_content_does_not_panic() {
        let m = parse_with_reference("<134>Aug 27 10:15:00 hôte app: Verbindung überprüfen", reference()).unwrap();
        assert!(m.message.contains("überprüfen"));
    }

    #[test]
    fn a_line_without_priority_is_refused() {
        assert!(parse_with_reference("Aug 27 10:15:00 host app: no priority", reference()).is_none());
    }
}
