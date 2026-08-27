//! Mikrotik RouterOS classification.
//!
//! RouterOS sends no hostname and puts its comma-separated topic list where
//! the hostname would be: `firewall,info forward: in:ether1 out:ether2, ...`.
//! The generic RFC 3164 parser therefore reads the topic list as the hostname,
//! which is what the shape check below relies on.
//!
//! Verification status: built from RouterOS documentation and published log
//! samples, not replayed against a live router.

use super::{find_mac, NetworkClassifier};
use crate::models::network_event::{
    NetworkEntities, NetworkEvent, NetworkEventType, NetworkVendor, Protocol,
};
use crate::normalizer::syslog::SyslogMessage;
use std::net::IpAddr;

pub struct MikrotikClassifier;

/// RouterOS topics that carry events this classifier understands.
const KNOWN_TOPICS: [&str; 5] = ["firewall", "wireless", "dhcp", "system", "interface"];

impl NetworkClassifier for MikrotikClassifier {
    fn vendor(&self) -> NetworkVendor {
        NetworkVendor::Mikrotik
    }

    fn matches(&self, msg: &SyslogMessage) -> bool {
        let topics = msg.hostname.as_deref().unwrap_or_default();
        topics.contains(',') && KNOWN_TOPICS.iter().any(|t| topics.starts_with(t))
    }

    fn classify(&self, msg: &SyslogMessage) -> Option<NetworkEvent> {
        let topics = msg.hostname.as_deref().unwrap_or_default();
        let topic = topics.split(',').next()?;
        let action = msg.app_name.as_deref().unwrap_or_default();

        match topic {
            "firewall" => Some(classify_firewall(action, &msg.message)),
            "wireless" => classify_wireless(action, &msg.message),
            "dhcp" => classify_dhcp(&msg.message),
            _ => None,
        }
    }
}

/// The chain action is the syslog tag: `forward:`, `input:`, `drop:`.
fn classify_firewall(action: &str, message: &str) -> NetworkEvent {
    let event_type = match action {
        "drop" | "reject" => NetworkEventType::FirewallBlocked,
        "accept" => NetworkEventType::FirewallAllowed,
        other => NetworkEventType::Other(format!("firewall_{other}")),
    };

    let entities = NetworkEntities {
        client_mac: find_mac(message),
        interface: read_labelled(message, "in:"),
        protocol: read_labelled(message, "proto ")
            .and_then(|p| Protocol::from_name(p.split_whitespace().next().unwrap_or(&p))),
        ..read_endpoints(message)
    };

    NetworkEvent::new(NetworkVendor::Mikrotik, event_type)
        .with_entities(entities)
        .with_vendor_key(format!("firewall/{action}"))
}

/// RouterOS writes the station as `aa:bb:cc:dd:ee:ff@wlan1` at the start of
/// the line, so the syslog tag holds the client rather than a daemon name.
fn classify_wireless(station: &str, message: &str) -> Option<NetworkEvent> {
    let event_type = if message.contains("rejected") || message.contains("failed authentication") {
        NetworkEventType::WifiAuthFailure
    } else if message.contains("disconnected") {
        NetworkEventType::WifiDisassociated
    } else if message.contains("connected") {
        NetworkEventType::WifiAssociated
    } else {
        return None;
    };

    let entities = NetworkEntities {
        client_mac: find_mac(station).or_else(|| find_mac(message)),
        interface: station.split('@').nth(1).map(|s| s.trim().to_string()),
        ..Default::default()
    };
    Some(
        NetworkEvent::new(NetworkVendor::Mikrotik, event_type)
            .with_entities(entities)
            .with_vendor_key("wireless"),
    )
}

fn classify_dhcp(message: &str) -> Option<NetworkEvent> {
    let event_type = if message.contains("deassigned") {
        NetworkEventType::Other("dhcp_lease_released".to_string())
    } else if message.contains("assigned") {
        NetworkEventType::DhcpLeaseGranted
    } else {
        return None;
    };

    let entities = NetworkEntities {
        client_mac: find_mac(message),
        src_ip: message
            .split_whitespace()
            .find_map(|t| t.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<IpAddr>().ok()),
        ..Default::default()
    };
    Some(
        NetworkEvent::new(NetworkVendor::Mikrotik, event_type)
            .with_entities(entities)
            .with_vendor_key("dhcp"),
    )
}

/// Reads `192.168.1.10:54321->1.2.3.4:443`, the RouterOS endpoint notation.
fn read_endpoints(message: &str) -> NetworkEntities {
    let mut entities = NetworkEntities::default();
    let Some(arrow) = message.find("->") else {
        return entities;
    };
    let left = message[..arrow].rsplit([' ', ',']).next().unwrap_or("");
    let right = message[arrow + 2..]
        .split([' ', ','])
        .next()
        .unwrap_or("");

    let (src_ip, src_port) = split_endpoint(left);
    let (dst_ip, dst_port) = split_endpoint(right);
    entities.src_ip = src_ip;
    entities.src_port = src_port;
    entities.dst_ip = dst_ip;
    entities.dst_port = dst_port;
    entities
}

/// Splits `host:port`, tolerating a bare address for protocols without ports.
fn split_endpoint(text: &str) -> (Option<IpAddr>, Option<u16>) {
    match text.rsplit_once(':') {
        Some((host, port)) => (host.parse().ok(), port.parse().ok()),
        None => (text.parse().ok(), None),
    }
}

/// Reads the value following a `label` up to the next space or comma.
fn read_labelled(message: &str, label: &str) -> Option<String> {
    let start = message.find(label)? + label.len();
    let rest = &message[start..];
    let end = rest.find([' ', ',']).unwrap_or(rest.len());
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_event::MacAddr;
    use crate::normalizer::syslog;

    fn classify(line: &str) -> Option<NetworkEvent> {
        MikrotikClassifier.classify(&syslog::parse(line)?)
    }

    #[test]
    fn a_dropped_forward_yields_both_endpoints() {
        let line = "<134>Aug 27 10:15:00 firewall,info drop: in:ether1 out:ether2, src-mac aa:bb:cc:dd:ee:ff, proto TCP (SYN), 192.168.1.10:54321->203.0.113.5:443, len 60";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::FirewallBlocked);
        assert_eq!(ev.entities.interface.as_deref(), Some("ether1"));
        assert_eq!(ev.entities.protocol, Some(Protocol::Tcp));
        assert_eq!(ev.entities.src_ip.unwrap().to_string(), "192.168.1.10");
        assert_eq!(ev.entities.src_port, Some(54321));
        assert_eq!(ev.entities.dst_ip.unwrap().to_string(), "203.0.113.5");
        assert_eq!(ev.entities.dst_port, Some(443));
        assert_eq!(ev.entities.client_mac, MacAddr::parse("aa:bb:cc:dd:ee:ff"));
    }

    #[test]
    fn an_accepted_packet_is_not_reported_as_blocked() {
        let line = "<134>Aug 27 10:15:00 firewall,info accept: in:ether1 out:ether2, proto UDP, 10.0.0.1:53124->8.8.8.8:53, len 70";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::FirewallAllowed);
        assert_eq!(ev.entities.protocol, Some(Protocol::Udp));
    }

    #[test]
    fn a_rejected_wireless_client_is_an_auth_failure() {
        let line = "<134>Aug 27 10:15:00 wireless,info aa:bb:cc:dd:ee:ff@wlan1: rejected, invalid key";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::WifiAuthFailure);
        assert_eq!(ev.entities.client_mac, MacAddr::parse("aa:bb:cc:dd:ee:ff"));
    }

    #[test]
    fn a_dhcp_assignment_is_recognized() {
        let line = "<134>Aug 27 10:15:00 dhcp,info defconf assigned 192.168.88.254 to aa:bb:cc:dd:ee:ff";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::DhcpLeaseGranted);
        assert_eq!(ev.entities.client_mac, MacAddr::parse("aa:bb:cc:dd:ee:ff"));
    }

    #[test]
    fn lines_from_other_vendors_are_not_claimed() {
        let unifi = syslog::parse("<134>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff IEEE 802.11: associated").unwrap();
        assert!(!MikrotikClassifier.matches(&unifi));
    }

    #[test]
    fn an_unknown_topic_yields_nothing() {
        let line = "<134>Aug 27 10:15:00 system,info user admin logged in from 10.0.0.5 via winbox";
        assert!(classify(line).is_none());
    }
}
