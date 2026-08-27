//! pfSense and OPNsense `filterlog` classification.
//!
//! OPNsense inherited pfSense's filterlog verbatim, so one parser serves
//! both. The payload is positional CSV whose field count depends on the IP
//! version and the transport protocol, which is why the offsets below are
//! derived rather than hard-coded past field 8.
//!
//! Reference: pfSense `filterlog` source, field order documented at
//! <https://docs.netgate.com/pfsense/en/latest/monitoring/logs/raw-filter-format.html>

use super::NetworkClassifier;
use crate::models::network_event::{
    NetworkEntities, NetworkEvent, NetworkEventType, NetworkVendor, Protocol,
};
use crate::normalizer::syslog::SyslogMessage;
use std::net::IpAddr;

pub struct PfSenseClassifier;

/// Field positions that are the same for every filterlog line.
const F_TRACKER: usize = 3;
const F_INTERFACE: usize = 4;
const F_ACTION: usize = 6;
const F_DIRECTION: usize = 7;
const F_IP_VERSION: usize = 8;

impl NetworkClassifier for PfSenseClassifier {
    fn vendor(&self) -> NetworkVendor {
        NetworkVendor::PfSense
    }

    fn matches(&self, msg: &SyslogMessage) -> bool {
        msg.app_name.as_deref() == Some("filterlog")
    }

    fn classify(&self, msg: &SyslogMessage) -> Option<NetworkEvent> {
        let fields: Vec<&str> = msg.message.split(',').collect();
        if fields.len() <= F_IP_VERSION {
            return None;
        }

        let event_type = match *fields.get(F_ACTION)? {
            "block" | "reject" => NetworkEventType::FirewallBlocked,
            "pass" => NetworkEventType::FirewallAllowed,
            other => NetworkEventType::Other(format!("firewall_{other}")),
        };

        let mut entities = NetworkEntities {
            interface: nonempty(fields[F_INTERFACE]),
            rule_id: nonempty(fields[F_TRACKER]),
            reason: nonempty(fields[F_DIRECTION]).map(|d| format!("direction {d}")),
            ..Default::default()
        };
        read_addresses(&fields, &mut entities);

        Some(
            NetworkEvent::new(NetworkVendor::PfSense, event_type)
                .with_entities(entities)
                .with_vendor_key("filterlog"),
        )
    }
}

/// Reads the address block, whose offset differs between IPv4 and IPv6
/// because the two carry a different number of header fields before it.
fn read_addresses(fields: &[&str], entities: &mut NetworkEntities) {
    let Some(offsets) = address_offsets(fields) else {
        return;
    };
    entities.protocol = fields
        .get(offsets.proto_name)
        .and_then(|p| Protocol::from_name(p))
        .or_else(|| {
            fields
                .get(offsets.proto_num)
                .and_then(|n| n.parse::<u8>().ok())
                .map(Protocol::from_number)
        });
    entities.src_ip = fields.get(offsets.src).and_then(|s| s.parse::<IpAddr>().ok());
    entities.dst_ip = fields.get(offsets.dst).and_then(|s| s.parse::<IpAddr>().ok());
    entities.src_port = fields.get(offsets.src + 2).and_then(|p| p.parse().ok());
    entities.dst_port = fields.get(offsets.dst + 2).and_then(|p| p.parse().ok());
}

struct AddressOffsets {
    proto_num: usize,
    proto_name: usize,
    src: usize,
    dst: usize,
}

/// IPv4 lines carry tos/ecn/ttl/id/offset/flags before the protocol; IPv6
/// lines carry class/flowlabel/hoplimit and name the protocol first.
fn address_offsets(fields: &[&str]) -> Option<AddressOffsets> {
    match *fields.get(F_IP_VERSION)? {
        "4" => Some(AddressOffsets { proto_num: 15, proto_name: 16, src: 18, dst: 19 }),
        "6" => Some(AddressOffsets { proto_num: 13, proto_name: 12, src: 15, dst: 16 }),
        _ => None,
    }
}

fn nonempty(field: &str) -> Option<String> {
    let trimmed = field.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::syslog;

    fn classify(line: &str) -> Option<NetworkEvent> {
        let msg = syslog::parse(line)?;
        PfSenseClassifier.classify(&msg)
    }

    #[test]
    fn an_ipv4_tcp_block_yields_the_full_five_tuple() {
        let line = "<134>Aug 27 10:15:00 firewall filterlog: 5,,,1000000103,igb0,match,block,in,4,0x0,,64,12345,0,DF,6,tcp,60,192.0.2.10,198.51.100.20,54321,443,0,S,1234567,0,64240,,mss";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::FirewallBlocked);
        assert_eq!(ev.entities.interface.as_deref(), Some("igb0"));
        assert_eq!(ev.entities.rule_id.as_deref(), Some("1000000103"));
        assert_eq!(ev.entities.protocol, Some(Protocol::Tcp));
        assert_eq!(ev.entities.src_ip.unwrap().to_string(), "192.0.2.10");
        assert_eq!(ev.entities.dst_ip.unwrap().to_string(), "198.51.100.20");
        assert_eq!(ev.entities.src_port, Some(54321));
        assert_eq!(ev.entities.dst_port, Some(443));
    }

    #[test]
    fn a_pass_action_is_not_reported_as_a_block() {
        let line = "<134>Aug 27 10:15:00 firewall filterlog: 2,,,1000000104,igb1,match,pass,out,4,0x0,,64,0,0,DF,17,udp,80,10.0.0.5,8.8.8.8,53124,53,60";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::FirewallAllowed);
        assert_eq!(ev.entities.protocol, Some(Protocol::Udp));
        assert_eq!(ev.entities.dst_port, Some(53));
    }

    #[test]
    fn an_ipv6_line_uses_the_shifted_address_offsets() {
        let line = "<134>Aug 27 10:15:00 firewall filterlog: 8,,,1000000105,igb0,match,block,in,6,0x00,0x00000,64,tcp,6,40,2001:db8::1,2001:db8::2,443,54321,0,S";
        let ev = classify(line).unwrap();
        assert_eq!(ev.event_type, NetworkEventType::FirewallBlocked);
        assert_eq!(ev.entities.src_ip.unwrap().to_string(), "2001:db8::1");
        assert_eq!(ev.entities.dst_ip.unwrap().to_string(), "2001:db8::2");
        assert_eq!(ev.entities.protocol, Some(Protocol::Tcp));
    }

    #[test]
    fn an_icmp_line_has_no_ports_but_keeps_its_addresses() {
        let line = "<134>Aug 27 10:15:00 firewall filterlog: 5,,,1000000103,igb0,match,block,in,4,0x0,,64,1,0,none,1,icmp,84,192.0.2.10,198.51.100.20,request,1,1";
        let ev = classify(line).unwrap();
        assert_eq!(ev.entities.protocol, Some(Protocol::Icmp));
        assert_eq!(ev.entities.src_port, None);
        assert!(ev.entities.dst_ip.is_some());
    }

    #[test]
    fn a_truncated_line_is_refused_instead_of_panicking() {
        let msg = syslog::parse("<134>Aug 27 10:15:00 firewall filterlog: 5,,,1000").unwrap();
        assert!(PfSenseClassifier.classify(&msg).is_none());
    }

    #[test]
    fn only_filterlog_lines_are_claimed() {
        let ours = syslog::parse("<134>Aug 27 10:15:00 fw filterlog: 5,,,1,igb0,match,block,in,4").unwrap();
        let theirs = syslog::parse("<134>Aug 27 10:15:00 fw dhcpd: lease granted").unwrap();
        assert!(PfSenseClassifier.matches(&ours));
        assert!(!PfSenseClassifier.matches(&theirs));
    }
}
