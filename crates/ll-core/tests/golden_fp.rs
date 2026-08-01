//! Haelt die Fingerabdruck-Ausgabe fest.
//!
//! Der Fingerabdruck steht in `log_clusters` in der Datenbank des Nutzers.
//! Aendert sich die Hex-Darstellung, passen bestehende Cluster nicht mehr zu
//! neu berechneten, und die Gruppierung faellt still auseinander. Der Wert
//! hier wurde unter sha2 0.10 gemessen, bevor auf 0.11 gehoben wurde.

use ll_core::clustering::fingerprint::compute_fingerprint;

#[test]
fn the_fingerprint_of_a_known_line_does_not_move() {
    let (fp, _) = compute_fingerprint(
        "ERROR 2026-01-01T00:00:00Z connection refused to 10.0.0.1 for user a@b.ch",
    );
    assert_eq!(fp, "2fa932c23d4ab7ce", "Fingerabdruecke in bestehenden Datenbanken wuerden nicht mehr passen");
}

#[test]
fn the_fingerprint_is_lowercase_hex_of_the_expected_width() {
    let (fp, _) = compute_fingerprint("irgendeine Zeile");
    assert_eq!(fp.len(), 16);
    assert!(fp.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}
