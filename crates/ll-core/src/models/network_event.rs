//! Network-domain view of a log entry.
//!
//! Generic log analysis treats a line as text plus a level. Network operations
//! need the entities inside that text as first-class values: which client, on
//! which access point, against which firewall rule. Extracting them once, at
//! parse time, is what makes correlation and AI explanation possible later.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::IpAddr;

/// A 48-bit hardware address, stored normalized so that `AA-BB-CC-DD-EE-FF`,
/// `aabb.ccdd.eeff` and `aa:bb:cc:dd:ee:ff` compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    pub fn new(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    pub fn octets(&self) -> [u8; 6] {
        self.0
    }

    /// Accepts colon, hyphen and Cisco dot notation. Returns `None` for
    /// anything that is not exactly six octets.
    pub fn parse(s: &str) -> Option<Self> {
        let hex: Vec<u8> = s
            .bytes()
            .filter(|b| b.is_ascii_hexdigit())
            .map(|b| (b as char).to_digit(16).unwrap_or(0) as u8)
            .collect();
        if hex.len() != 12 {
            return None;
        }
        let mut octets = [0u8; 6];
        for (i, pair) in hex.chunks_exact(2).enumerate() {
            octets[i] = (pair[0] << 4) | pair[1];
        }
        Some(Self(octets))
    }

    /// The IEEE OUI, the vendor half of the address.
    pub fn oui(&self) -> [u8; 3] {
        [self.0[0], self.0[1], self.0[2]]
    }

    /// Locally administered addresses are what iOS and Android emit as
    /// randomized WiFi MACs, so a "new device" that carries this bit is
    /// usually a known device in disguise rather than an intruder.
    pub fn is_locally_administered(&self) -> bool {
        self.0[0] & 0b0000_0010 != 0
    }

    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0b0000_0001 != 0
    }

    /// Keeps the vendor OUI and replaces the device half, so shared output
    /// stays diagnostically useful without identifying a specific device.
    pub fn anonymized(&self) -> Self {
        Self([self.0[0], self.0[1], self.0[2], 0x00, 0x00, 0x00])
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = self.0;
        write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", o[0], o[1], o[2], o[3], o[4], o[5])
    }
}

/// IP-layer protocol, kept as an enum for the handful that carry meaning in
/// firewall logs and as a number for everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Icmpv6,
    Esp,
    Ah,
    Gre,
    Other(u8),
}

impl Protocol {
    pub fn from_number(n: u8) -> Self {
        match n {
            1 => Self::Icmp,
            6 => Self::Tcp,
            17 => Self::Udp,
            50 => Self::Esp,
            51 => Self::Ah,
            47 => Self::Gre,
            58 => Self::Icmpv6,
            other => Self::Other(other),
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "tcp" => Some(Self::Tcp),
            "udp" => Some(Self::Udp),
            "icmp" => Some(Self::Icmp),
            "icmpv6" | "ipv6-icmp" => Some(Self::Icmpv6),
            "esp" => Some(Self::Esp),
            "ah" => Some(Self::Ah),
            "gre" => Some(Self::Gre),
            _ => None,
        }
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tcp => write!(f, "tcp"),
            Self::Udp => write!(f, "udp"),
            Self::Icmp => write!(f, "icmp"),
            Self::Icmpv6 => write!(f, "icmpv6"),
            Self::Esp => write!(f, "esp"),
            Self::Ah => write!(f, "ah"),
            Self::Gre => write!(f, "gre"),
            Self::Other(n) => write!(f, "proto{n}"),
        }
    }
}

/// What actually happened, independent of which vendor phrased it how.
///
/// The variants are the ones an operator triages by. Anything a parser cannot
/// place lands in `Other`, carrying the vendor's own label, so an unknown
/// event is still searchable instead of being silently discarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkEventType {
    WifiAuthFailure,
    WifiAssociated,
    WifiDisassociated,
    WifiRoamed,
    DhcpLeaseGranted,
    DhcpLeaseDenied,
    FirewallBlocked,
    FirewallAllowed,
    IdsAlert,
    VpnConnected,
    VpnDisconnected,
    PortLinkUp,
    PortLinkDown,
    PoeFault,
    StpTopologyChange,
    DeviceAdopted,
    DeviceRestarted,
    DeviceUnreachable,
    DnsBlocked,
    AuthenticationFailure,
    ConfigurationChanged,
    Other(String),
}

impl NetworkEventType {
    /// Whether this event class warrants operator attention on its own.
    /// A single blocked packet is noise; a failed authentication is not.
    pub fn is_noteworthy(&self) -> bool {
        matches!(
            self,
            Self::WifiAuthFailure
                | Self::DhcpLeaseDenied
                | Self::IdsAlert
                | Self::PoeFault
                | Self::PortLinkDown
                | Self::DeviceUnreachable
                | Self::AuthenticationFailure
                | Self::StpTopologyChange
        )
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::WifiAuthFailure => "wifi_auth_failure",
            Self::WifiAssociated => "wifi_associated",
            Self::WifiDisassociated => "wifi_disassociated",
            Self::WifiRoamed => "wifi_roamed",
            Self::DhcpLeaseGranted => "dhcp_lease_granted",
            Self::DhcpLeaseDenied => "dhcp_lease_denied",
            Self::FirewallBlocked => "firewall_blocked",
            Self::FirewallAllowed => "firewall_allowed",
            Self::IdsAlert => "ids_alert",
            Self::VpnConnected => "vpn_connected",
            Self::VpnDisconnected => "vpn_disconnected",
            Self::PortLinkUp => "port_link_up",
            Self::PortLinkDown => "port_link_down",
            Self::PoeFault => "poe_fault",
            Self::StpTopologyChange => "stp_topology_change",
            Self::DeviceAdopted => "device_adopted",
            Self::DeviceRestarted => "device_restarted",
            Self::DeviceUnreachable => "device_unreachable",
            Self::DnsBlocked => "dns_blocked",
            Self::AuthenticationFailure => "authentication_failure",
            Self::ConfigurationChanged => "configuration_changed",
            Self::Other(s) => s,
        }
    }
}

/// Which product family produced the line. Kept separate from the parser id
/// because several vendors share one parser (OPNsense reuses pfSense's
/// filterlog format) and one vendor can need several parsers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkVendor {
    Unifi,
    Uisp,
    PfSense,
    OpnSense,
    Mikrotik,
    GenericSyslog,
}

impl NetworkVendor {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Unifi => "unifi",
            Self::Uisp => "uisp",
            Self::PfSense => "pfsense",
            Self::OpnSense => "opnsense",
            Self::Mikrotik => "mikrotik",
            Self::GenericSyslog => "syslog",
        }
    }
}

/// The entities a network event refers to.
///
/// Every field is optional because no single log format fills all of them;
/// a WiFi association has a client MAC and an SSID but no ports, a firewall
/// block has a five-tuple but no SSID.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkEntities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_mac: Option<MacAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_mac: Option<MacAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_ip: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst_ip: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Protocol>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_band: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl NetworkEntities {
    /// True when nothing was extracted, which tells the caller the parser
    /// recognized the line shape but found no entities worth indexing.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Replaces device-identifying values with stable, non-identifying
    /// stand-ins. Used for the sample data shipped with the project and for
    /// any output the operator intends to share.
    pub fn anonymized(&self) -> Self {
        Self {
            client_mac: self.client_mac.map(|m| m.anonymized()),
            device_mac: self.device_mac.map(|m| m.anonymized()),
            ssid: self.ssid.as_ref().map(|_| "example-ssid".to_string()),
            ..self.clone()
        }
    }
}

/// The network-domain classification attached to a log entry.
///
/// This does not replace `NormalizedEntry`; it rides alongside it, so generic
/// features (search, clustering, timeline) keep working unchanged on lines
/// that no network parser understood.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkEvent {
    pub vendor: NetworkVendor,
    pub event_type: NetworkEventType,
    pub entities: NetworkEntities,
    /// The vendor's own event key where one exists, kept verbatim so an
    /// operator can search for what the vendor's own documentation calls it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_event_key: Option<String>,
}

impl NetworkEvent {
    pub fn new(vendor: NetworkVendor, event_type: NetworkEventType) -> Self {
        Self { vendor, event_type, entities: NetworkEntities::default(), vendor_event_key: None }
    }

    pub fn with_entities(mut self, entities: NetworkEntities) -> Self {
        self.entities = entities;
        self
    }

    pub fn with_vendor_key(mut self, key: impl Into<String>) -> Self {
        self.vendor_event_key = Some(key.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mac_parses_all_three_notations_to_the_same_value() {
        let colon = MacAddr::parse("aa:bb:cc:dd:ee:ff").unwrap();
        let hyphen = MacAddr::parse("AA-BB-CC-DD-EE-FF").unwrap();
        let cisco = MacAddr::parse("aabb.ccdd.eeff").unwrap();
        assert_eq!(colon, hyphen);
        assert_eq!(colon, cisco);
        assert_eq!(colon.to_string(), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn mac_rejects_wrong_length() {
        assert!(MacAddr::parse("aa:bb:cc:dd:ee").is_none());
        assert!(MacAddr::parse("aa:bb:cc:dd:ee:ff:00").is_none());
        assert!(MacAddr::parse("").is_none());
    }

    #[test]
    fn randomized_client_mac_is_recognized() {
        // iOS private WiFi address: bit 1 of the first octet is set.
        let randomized = MacAddr::parse("a2:bb:cc:dd:ee:ff").unwrap();
        let burned_in = MacAddr::parse("a0:bb:cc:dd:ee:ff").unwrap();
        assert!(randomized.is_locally_administered());
        assert!(!burned_in.is_locally_administered());
    }

    #[test]
    fn anonymizing_keeps_the_vendor_and_drops_the_device() {
        let mac = MacAddr::parse("74:ac:b9:12:34:56").unwrap();
        let anon = mac.anonymized();
        assert_eq!(anon.oui(), mac.oui());
        assert_eq!(anon.to_string(), "74:ac:b9:00:00:00");
    }

    #[test]
    fn protocol_maps_numbers_and_names_consistently() {
        assert_eq!(Protocol::from_number(6), Protocol::Tcp);
        assert_eq!(Protocol::from_name("TCP"), Some(Protocol::Tcp));
        assert_eq!(Protocol::from_number(253), Protocol::Other(253));
        assert_eq!(Protocol::Other(253).to_string(), "proto253");
    }

    #[test]
    fn unknown_vendor_events_keep_their_label() {
        let ev = NetworkEventType::Other("wlan_channel_changed".into());
        assert_eq!(ev.as_str(), "wlan_channel_changed");
        assert!(!ev.is_noteworthy());
    }

    #[test]
    fn empty_entities_serialize_without_null_noise() {
        let ev = NetworkEvent::new(NetworkVendor::Unifi, NetworkEventType::WifiAuthFailure);
        let json = serde_json::to_string(&ev).unwrap();
        assert!(!json.contains("null"), "optional fields must be skipped, got {json}");
    }
}
