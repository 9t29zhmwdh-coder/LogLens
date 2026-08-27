//! Vendor classification: turning a parsed syslog line into a network event.
//!
//! The syslog layer answers "what shape is this line". This layer answers
//! "what happened", which is vendor-specific: a failed WPA2 handshake is a
//! `WPA: invalid MIC` on a UniFi access point and a `block` action on a
//! pfSense filter rule, and both must land on the same event type for
//! correlation to work.

pub mod mikrotik;
pub mod pfsense;
pub mod unifi;

use crate::models::network_event::{NetworkEvent, NetworkVendor};
use crate::normalizer::syslog::SyslogMessage;
use std::sync::Arc;

/// A vendor-specific interpreter for parsed syslog messages.
pub trait NetworkClassifier: Send + Sync {
    fn vendor(&self) -> NetworkVendor;

    /// Cheap shape check used to pick a classifier without running the full
    /// extraction. Must not allocate; it runs against every line.
    fn matches(&self, msg: &SyslogMessage) -> bool;

    /// Extracts the event. Returning `None` after `matches` said yes is
    /// legitimate: the vendor is right but this particular line carries
    /// nothing worth classifying.
    fn classify(&self, msg: &SyslogMessage) -> Option<NetworkEvent>;
}

/// Holds the classifiers and picks the first whose shape check passes.
pub struct ClassifierRegistry {
    classifiers: Vec<Arc<dyn NetworkClassifier>>,
}

impl ClassifierRegistry {
    /// Empty registry, for tests that want one specific classifier.
    pub fn empty() -> Self {
        Self { classifiers: Vec::new() }
    }

    /// Every classifier shipped with the project.
    pub fn with_builtins() -> Self {
        let mut registry = Self::empty();
        registry.register(Arc::new(unifi::UnifiClassifier));
        registry.register(Arc::new(pfsense::PfSenseClassifier));
        registry.register(Arc::new(mikrotik::MikrotikClassifier));
        registry
    }

    pub fn register(&mut self, classifier: Arc<dyn NetworkClassifier>) {
        self.classifiers.push(classifier);
    }

    pub fn classify(&self, msg: &SyslogMessage) -> Option<NetworkEvent> {
        self.classifiers
            .iter()
            .find(|c| c.matches(msg))
            .and_then(|c| c.classify(msg))
    }

    pub fn len(&self) -> usize {
        self.classifiers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classifiers.is_empty()
    }
}

impl Default for ClassifierRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Finds the first MAC address in a line. Shared because every vendor writes
/// one somewhere and none of them agree on where.
pub(crate) fn find_mac(text: &str) -> Option<crate::models::network_event::MacAddr> {
    use crate::models::network_event::MacAddr;
    text.split(|c: char| !(c.is_ascii_hexdigit() || c == ':' || c == '-' || c == '.'))
        .filter(|token| is_mac_shaped(token))
        .find_map(MacAddr::parse)
}

/// Guards against reading `10.0.0.1:5432` as a MAC: only the three canonical
/// notations have the right length, and a dotted token must be Cisco-style
/// with exactly two dots.
fn is_mac_shaped(token: &str) -> bool {
    match token.len() {
        12 => !token.contains([':', '-', '.']),
        14 => token.matches('.').count() == 2 && !token.contains([':', '-']),
        17 => token.matches(':').count() == 5 || token.matches('-').count() == 5,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalizer::syslog;

    #[test]
    fn the_builtin_registry_routes_each_vendor_to_its_own_classifier() {
        let registry = ClassifierRegistry::with_builtins();
        let cases = [
            ("<132>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff WPA: invalid MIC in msg 2/4 of 4-Way Handshake", NetworkVendor::Unifi),
            ("<134>Aug 27 10:15:00 fw filterlog: 5,,,1000000103,igb0,match,block,in,4,0x0,,64,1,0,DF,6,tcp,60,192.0.2.10,198.51.100.20,54321,443", NetworkVendor::PfSense),
            ("<134>Aug 27 10:15:00 firewall,info drop: in:ether1 out:ether2, proto TCP, 192.168.1.10:54321->203.0.113.5:443, len 60", NetworkVendor::Mikrotik),
        ];
        for (line, expected) in cases {
            let msg = syslog::parse(line).unwrap_or_else(|| panic!("unparseable: {line}"));
            let event = registry.classify(&msg).unwrap_or_else(|| panic!("unclassified: {line}"));
            assert_eq!(event.vendor, expected, "wrong vendor for: {line}");
        }
    }

    #[test]
    fn a_line_no_classifier_claims_returns_none() {
        let registry = ClassifierRegistry::with_builtins();
        let msg = syslog::parse("<134>Aug 27 10:15:00 server nginx: GET /index.html 200").unwrap();
        assert!(registry.classify(&msg).is_none());
    }

    #[test]
    fn an_empty_registry_classifies_nothing() {
        let registry = ClassifierRegistry::empty();
        assert!(registry.is_empty());
        let msg = syslog::parse("<134>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff IEEE 802.11: associated").unwrap();
        assert!(registry.classify(&msg).is_none());
    }

    #[test]
    fn find_mac_accepts_the_three_notations_and_rejects_lookalikes() {
        assert!(find_mac("STA aa:bb:cc:dd:ee:ff associated").is_some());
        assert!(find_mac("mac AA-BB-CC-DD-EE-FF up").is_some());
        assert!(find_mac("addr aabb.ccdd.eeff seen").is_some());
        assert!(find_mac("SRC=10.0.0.1 SPT=5432").is_none());
        assert!(find_mac("no address at all").is_none());
    }
}
