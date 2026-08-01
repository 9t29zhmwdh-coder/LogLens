use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Sha256, Digest};
use std::fmt::Write as _;

// Patterns to normalize away before hashing
static STRIP_UUID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").unwrap()
});
static STRIP_HEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b0x[0-9a-fA-F]{4,}\b").unwrap()
});
static STRIP_NUMBER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d+\b").unwrap()
});
static STRIP_ISO_TS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}[^\s]*").unwrap()
});
static STRIP_IP: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?\b").unwrap()
});
static STRIP_PATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"/(?:tmp|var|home|users|root)/[^\s\"']+"#).unwrap()
});
static STRIP_EMAIL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap()
});

/// Returns a stable fingerprint + normalized template for a log message.
pub fn compute_fingerprint(message: &str) -> (String, String) {
    let template = normalize(message);
    let mut hasher = Sha256::new();
    hasher.update(template.as_bytes());
    // sha2 0.11 liefert einen Typ ohne LowerHex, `format!("{:x}", ..)` geht
    // nicht mehr. Von Hand formatiert, weil die Ausgabe zeichengleich bleiben
    // muss: der Fingerabdruck steht in der Datenbank des Nutzers, und eine
    // andere Darstellung liesse bestehende Cluster ins Leere laufen.
    let digest = hasher.finalize();
    let mut hash = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(hash, "{byte:02x}").expect("Schreiben in einen String schlaegt nicht fehl");
    }
    let fingerprint = hash[..16].to_string();
    (fingerprint, template)
}

fn normalize(s: &str) -> String {
    let s = STRIP_ISO_TS.replace_all(s, "<TIMESTAMP>");
    let s = STRIP_UUID.replace_all(&s, "<UUID>");
    let s = STRIP_IP.replace_all(&s, "<IP>");
    let s = STRIP_EMAIL.replace_all(&s, "<EMAIL>");
    let s = STRIP_HEX.replace_all(&s, "<HEX>");
    let s = STRIP_PATH.replace_all(&s, "<PATH>");
    let s = STRIP_NUMBER.replace_all(&s, "<N>");
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_message_same_fingerprint() {
        let a = "Connection refused to 192.168.1.42:5432 after 3 retries";
        let b = "Connection refused to 10.0.0.1:5432 after 7 retries";
        let (fp_a, _) = compute_fingerprint(a);
        let (fp_b, _) = compute_fingerprint(b);
        assert_eq!(fp_a, fp_b);
    }

    #[test]
    fn different_errors_different_fingerprint() {
        let a = "Connection refused to 192.168.1.42:5432";
        let b = "Timeout waiting for lock on table users";
        let (fp_a, _) = compute_fingerprint(a);
        let (fp_b, _) = compute_fingerprint(b);
        assert_ne!(fp_a, fp_b);
    }

    #[test]
    fn uuid_stripped() {
        let msg = "Task 550e8400-e29b-41d4-a716-446655440000 failed";
        let (_, template) = compute_fingerprint(msg);
        assert!(template.contains("<UUID>"));
        assert!(!template.contains("550e8400"));
    }
}
