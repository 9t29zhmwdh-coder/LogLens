//! UniFi classification.
//!
//! A UniFi site does not have one log format. Access points speak through
//! `hostapd`, the gateway through `dnsmasq` for DHCP and through the kernel's
//! netfilter logging for firewall rules, and each writes its entities in a
//! different place. The classifier dispatches on the syslog tag first and
//! only then reads the message.
//!
//! Verification status: these patterns are built from Ubiquiti's published
//! log samples and the underlying upstream projects (hostapd, dnsmasq,
//! netfilter), which is where the wording originates. They have not been
//! replayed against a live controller's syslog stream. Treat additions here
//! as needing a captured sample in `tests/fixtures/` before being trusted.

use super::{find_mac, NetworkClassifier};
use crate::models::network_event::{
    NetworkEntities, NetworkEvent, NetworkEventType, NetworkVendor, Protocol,
};
use crate::normalizer::syslog::SyslogMessage;
use std::net::IpAddr;

pub struct UnifiClassifier;

impl NetworkClassifier for UnifiClassifier {
    fn vendor(&self) -> NetworkVendor {
        NetworkVendor::Unifi
    }

    fn matches(&self, msg: &SyslogMessage) -> bool {
        // The gateway tags DHCP lines `dnsmasq-dhcp`, not `dnsmasq`.
        msg.app_name.as_deref().is_some_and(|tag| {
            tag == "hostapd" || tag == "kernel" || tag.starts_with("dnsmasq")
        })
    }

    fn classify(&self, msg: &SyslogMessage) -> Option<NetworkEvent> {
        let tag = msg.app_name.as_deref()?;
        match tag {
            "hostapd" => classify_hostapd(&msg.message),
            "kernel" => classify_netfilter(&msg.message),
            _ if tag.starts_with("dnsmasq") => classify_dnsmasq(&msg.message),
            _ => None,
        }
    }
}

/// hostapd lines carry the radio interface, the station MAC and a verdict:
/// `ath0: STA aa:bb:.. IEEE 802.11: associated`
fn classify_hostapd(message: &str) -> Option<NetworkEvent> {
    let client_mac = find_mac(message);
    let interface = message.split(':').next().map(str::to_string);

    let (event_type, reason) = if message.contains("invalid MIC")
        || message.contains("possible PSK mismatch")
    {
        // A wrong pre-shared key surfaces as a failed 4-way handshake, not as
        // an explicit "wrong password" message.
        (NetworkEventType::WifiAuthFailure, Some("invalid_psk".to_string()))
    } else if message.contains("authentication failed") || message.contains("WPA: reject") {
        (NetworkEventType::WifiAuthFailure, Some("rejected".to_string()))
    } else if message.contains("deauthenticated") {
        (NetworkEventType::WifiDisassociated, deauth_reason(message))
    } else if message.contains("disassociated") {
        (NetworkEventType::WifiDisassociated, None)
    } else if message.contains("associated") {
        (NetworkEventType::WifiAssociated, None)
    } else if message.contains("handshake completed") {
        (NetworkEventType::WifiAssociated, Some("handshake_completed".to_string()))
    } else {
        return None;
    };

    let entities = NetworkEntities {
        client_mac,
        interface,
        radio_band: interface_band(message),
        reason,
        ..Default::default()
    };
    Some(
        NetworkEvent::new(NetworkVendor::Unifi, event_type)
            .with_entities(entities)
            .with_vendor_key("hostapd"),
    )
}

fn deauth_reason(message: &str) -> Option<String> {
    if message.contains("local deauth request") {
        Some("local_deauth".to_string())
    } else if message.contains("inactivity") {
        Some("inactivity".to_string())
    } else {
        None
    }
}

/// UniFi names radios by interface: ath0/ra0 are 2.4 GHz, ath1/rai0 5 GHz.
/// The mapping is firmware-dependent, so an unknown name yields nothing
/// rather than a guess.
fn interface_band(message: &str) -> Option<String> {
    let interface = message.split(':').next()?;
    match interface {
        "ath0" | "ra0" => Some("2.4GHz".to_string()),
        "ath1" | "rai0" => Some("5GHz".to_string()),
        "ath2" | "rax0" => Some("6GHz".to_string()),
        _ => None,
    }
}

/// dnsmasq DHCP lines: `DHCPACK(br0) 192.168.1.50 aa:bb:.. hostname`
fn classify_dnsmasq(message: &str) -> Option<NetworkEvent> {
    let verb = message.split('(').next()?.trim();
    let event_type = match verb {
        "DHCPACK" => NetworkEventType::DhcpLeaseGranted,
        "DHCPNAK" | "DHCPDECLINE" => NetworkEventType::DhcpLeaseDenied,
        _ => return None,
    };

    let interface = message
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(iface, _)| iface.to_string());

    let entities = NetworkEntities {
        client_mac: find_mac(message),
        src_ip: message.split_whitespace().find_map(|t| t.parse::<IpAddr>().ok()),
        interface,
        reason: (event_type == NetworkEventType::DhcpLeaseDenied)
            .then(|| trailing_reason(message))
            .flatten(),
        ..Default::default()
    };
    Some(
        NetworkEvent::new(NetworkVendor::Unifi, event_type)
            .with_entities(entities)
            .with_vendor_key(verb),
    )
}

/// The rejection reason is the free text after the client's MAC address,
/// which is the last structured field dnsmasq writes.
fn trailing_reason(message: &str) -> Option<String> {
    let mac = find_mac(message)?;
    let position = message.to_ascii_lowercase().find(&mac.to_string())?;
    let tail = message[position + mac.to_string().len()..].trim();
    (!tail.is_empty()).then(|| tail.to_string())
}

/// netfilter lines prefixed by the UniFi rule name:
/// `[LAN_IN-4-D]IN=br0 OUT=eth0 SRC=.. DST=.. PROTO=TCP SPT=.. DPT=..`
fn classify_netfilter(message: &str) -> Option<NetworkEvent> {
    let rule_id = message
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .map(|(rule, _)| rule.to_string());

    let payload = message
        .split_once(']')
        .map(|(_, rest)| rest)
        .unwrap_or(message);
    let fields = netfilter_fields(payload);
    if fields.is_empty() {
        return None;
    }

    // UniFi encodes the verdict in the rule name's trailing letter:
    // D for drop, A for accept, R for reject.
    let event_type = match rule_id.as_deref().and_then(|r| r.rsplit('-').next()) {
        Some("D") | Some("R") => NetworkEventType::FirewallBlocked,
        Some("A") => NetworkEventType::FirewallAllowed,
        _ => NetworkEventType::Other("firewall_logged".to_string()),
    };

    let entities = NetworkEntities {
        src_ip: field(&fields, "SRC").and_then(|v| v.parse().ok()),
        dst_ip: field(&fields, "DST").and_then(|v| v.parse().ok()),
        src_port: field(&fields, "SPT").and_then(|v| v.parse().ok()),
        dst_port: field(&fields, "DPT").and_then(|v| v.parse().ok()),
        protocol: field(&fields, "PROTO").and_then(|v| Protocol::from_name(&v)),
        interface: field(&fields, "IN").filter(|v| !v.is_empty()),
        rule_id,
        ..Default::default()
    };
    Some(
        NetworkEvent::new(NetworkVendor::Unifi, event_type)
            .with_entities(entities)
            .with_vendor_key("netfilter"),
    )
}

fn netfilter_fields(message: &str) -> Vec<(&str, &str)> {
    message
        .split_whitespace()
        .filter_map(|token| token.split_once('='))
        .collect()
}

fn field(fields: &[(&str, &str)], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::network_event::MacAddr;
    use crate::normalizer::syslog;

    fn classify(line: &str) -> Option<NetworkEvent> {
        UnifiClassifier.classify(&syslog::parse(line)?)
    }

    #[test]
    fn a_wrong_wifi_password_surfaces_as_an_auth_failure() {
        let line = "<132>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff WPA: invalid MIC in msg 2/4 of 4-Way Handshake";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::WifiAuthFailure);
        assert_eq!(ev.entities.reason.as_deref(), Some("invalid_psk"));
        assert_eq!(ev.entities.client_mac, MacAddr::parse("aa:bb:cc:dd:ee:ff"));
        assert_eq!(ev.entities.interface.as_deref(), Some("ath0"));
        assert_eq!(ev.entities.radio_band.as_deref(), Some("2.4GHz"));
    }

    #[test]
    fn disassociated_is_not_swallowed_by_the_associated_check() {
        // "disassociated" contains "associated" as a substring; order matters.
        let line = "<134>Aug 27 10:15:00 U7-Pro hostapd: ath1: STA aa:bb:cc:dd:ee:ff IEEE 802.11: disassociated";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::WifiDisassociated);
        assert_eq!(ev.entities.radio_band.as_deref(), Some("5GHz"));
    }

    #[test]
    fn a_local_deauth_records_why_it_happened() {
        let line = "<134>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff IEEE 802.11: deauthenticated due to local deauth request";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::WifiDisassociated);
        assert_eq!(ev.entities.reason.as_deref(), Some("local_deauth"));
    }

    #[test]
    fn a_successful_association_is_recognized() {
        let line = "<134>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff IEEE 802.11: associated";
        assert_eq!(classify(line).unwrap().event_type, NetworkEventType::WifiAssociated);
    }

    #[test]
    fn a_dhcp_ack_from_the_dhcp_tagged_daemon_is_read() {
        // The gateway tags these `dnsmasq-dhcp`, not `dnsmasq`.
        let line = "<134>Aug 27 10:15:00 UXG dnsmasq-dhcp[1234]: DHCPACK(br0) 192.168.1.50 aa:bb:cc:dd:ee:ff laptop";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::DhcpLeaseGranted);
        assert_eq!(ev.entities.src_ip.unwrap().to_string(), "192.168.1.50");
        assert_eq!(ev.entities.client_mac, MacAddr::parse("aa:bb:cc:dd:ee:ff"));
        assert_eq!(ev.entities.interface.as_deref(), Some("br0"));
    }

    #[test]
    fn a_dhcp_nak_carries_the_rejection_reason() {
        let line = "<134>Aug 27 10:15:00 UXG dnsmasq-dhcp[1234]: DHCPNAK(br0) 192.168.1.50 aa:bb:cc:dd:ee:ff wrong network";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::DhcpLeaseDenied);
        assert_eq!(ev.entities.reason.as_deref(), Some("wrong network"));
    }

    #[test]
    fn a_dropped_firewall_rule_yields_the_five_tuple() {
        let line = "<4>Aug 27 10:15:00 UXG kernel: [LAN_IN-4-D]IN=br0 OUT=eth0 SRC=192.168.1.10 DST=8.8.8.8 LEN=60 PROTO=TCP SPT=54321 DPT=443";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::FirewallBlocked);
        assert_eq!(ev.entities.rule_id.as_deref(), Some("LAN_IN-4-D"));
        // The rule prefix must not corrupt the first key/value pair.
        assert_eq!(ev.entities.interface.as_deref(), Some("br0"));
        assert_eq!(ev.entities.src_ip.unwrap().to_string(), "192.168.1.10");
        assert_eq!(ev.entities.dst_port, Some(443));
        assert_eq!(ev.entities.protocol, Some(Protocol::Tcp));
    }

    #[test]
    fn an_accept_rule_is_distinguished_from_a_drop() {
        let line = "<4>Aug 27 10:15:00 UXG kernel: [LAN_LOCAL-2-A]IN=br0 OUT= SRC=192.168.1.10 DST=192.168.1.1 PROTO=UDP SPT=5353 DPT=53";
        assert_eq!(classify(line).unwrap().event_type, NetworkEventType::FirewallAllowed);
    }

    #[test]
    fn an_ip_with_a_port_is_not_mistaken_for_a_mac_address() {
        let line = "<4>Aug 27 10:15:00 UXG kernel: [LAN_IN-4-D]IN=br0 SRC=10.0.0.1 DST=10.0.0.2 PROTO=TCP SPT=5432 DPT=443";
        assert_eq!(classify(line).unwrap().entities.client_mac, None);
    }

    #[test]
    fn unrelated_daemons_are_left_alone() {
        let msg = syslog::parse("<134>Aug 27 10:15:00 UXG sshd[900]: Accepted publickey").unwrap();
        assert!(!UnifiClassifier.matches(&msg));
    }

    #[test]
    fn a_recognized_daemon_with_an_unknown_message_yields_nothing() {
        let line = "<134>Aug 27 10:15:00 U7-Pro hostapd: ath0: CTRL-EVENT-SCAN-STARTED";
        assert!(classify(line).is_none());
    }
}
