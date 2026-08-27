//! Haelt fest, dass ein gruppierter Eintrag auch als Cluster in der Datenbank
//! landet.
//!
//! `upsert_cluster` existierte, wurde aber von niemandem aufgerufen. Die
//! Gruppierung lief also im Speicher, und die Cluster-Ansicht, die die Tabelle
//! liest, blieb in jeder ausgelieferten Fassung leer. Das faellt beim
//! Kompilieren nicht auf und in der Oberflaeche nur dann, wenn man weiss, dass
//! dort etwas stehen muesste.

use std::sync::Arc;

use ll_core::clustering::ClusterGrouper;
use ll_core::db;
use ll_core::models::log_entry::{LogFormat, LogLevel, LogSource, LogSourceKind, NormalizedEntry};
use chrono::Utc;

fn eintrag(nachricht: &str, quelle: &LogSource) -> NormalizedEntry {
    NormalizedEntry {
        id: uuid::Uuid::new_v4().to_string(),
        source_id: quelle.id.clone(),
        source_label: quelle.label.clone(),
        timestamp: Utc::now(),
        level: LogLevel::Warn,
        service: Some("hostapd".into()),
        message: nachricht.to_string(),
        stacktrace: None,
        fields: serde_json::json!({}),
        raw: nachricht.to_string(),
        format: LogFormat::Syslog,
        fingerprint: String::new(),
        cluster_id: None,
        ingested_at: Utc::now(),
    }
}

#[tokio::test]
async fn wiederholte_meldungen_werden_als_ein_cluster_gespeichert() {
    let verzeichnis = std::env::temp_dir().join(format!("ll-cluster-{}", std::process::id()));
    std::fs::create_dir_all(&verzeichnis).expect("Testverzeichnis");
    let pfad = verzeichnis.join("test.db");
    let _ = std::fs::remove_file(&pfad);

    let pool = db::open(&pfad).await.expect("Datenbank");
    let quelle = LogSource::new("Test", LogSourceKind::Stdin);
    db::queries::insert_source(&pool, &quelle).await.expect("Quelle");

    let grouper = Arc::new(ClusterGrouper::new());

    // Dieselbe Meldung mit wechselnder MAC: der Fingerabdruck streift die
    // veraenderlichen Teile ab, es bleibt ein Muster.
    for n in 0..12 {
        let mut e = eintrag(
            &format!("ath0: STA aa:bb:cc:dd:ee:{n:02x} WPA: invalid MIC in msg 2/4 of 4-Way Handshake"),
            &quelle,
        );
        let cluster_id = grouper.process(&mut e);
        db::queries::insert_entry(&pool, &e).await.expect("Eintrag");
        let cluster = grouper.get_cluster(&cluster_id).expect("Cluster im Speicher");
        db::queries::upsert_cluster(&pool, &cluster).await.expect("Cluster speichern");
    }

    let cluster = db::queries::list_clusters(&pool, 100).await.expect("Cluster lesen");
    assert_eq!(cluster.len(), 1, "zwoelf gleichartige Meldungen ergeben einen Cluster");
    assert_eq!(cluster[0].count, 12, "der Zaehler muss alle zwoelf enthalten");

    let _ = std::fs::remove_file(&pfad);
}

#[tokio::test]
async fn verschiedene_meldungen_bleiben_getrennte_cluster() {
    let verzeichnis = std::env::temp_dir().join(format!("ll-cluster2-{}", std::process::id()));
    std::fs::create_dir_all(&verzeichnis).expect("Testverzeichnis");
    let pfad = verzeichnis.join("test2.db");
    let _ = std::fs::remove_file(&pfad);

    let pool = db::open(&pfad).await.expect("Datenbank");
    let quelle = LogSource::new("Test", LogSourceKind::Stdin);
    db::queries::insert_source(&pool, &quelle).await.expect("Quelle");

    let grouper = Arc::new(ClusterGrouper::new());
    for nachricht in [
        "ath0: STA aa:bb:cc:dd:ee:ff WPA: invalid MIC in msg 2/4 of 4-Way Handshake",
        "DHCPACK(br0) 192.0.2.50 aa:bb:cc:dd:ee:ff geraet",
    ] {
        let mut e = eintrag(nachricht, &quelle);
        let cluster_id = grouper.process(&mut e);
        db::queries::insert_entry(&pool, &e).await.expect("Eintrag");
        let cluster = grouper.get_cluster(&cluster_id).expect("Cluster");
        db::queries::upsert_cluster(&pool, &cluster).await.expect("Cluster speichern");
    }

    let cluster = db::queries::list_clusters(&pool, 100).await.expect("Cluster lesen");
    assert_eq!(cluster.len(), 2, "unterschiedliche Muster duerfen nicht zusammenfallen");

    let _ = std::fs::remove_file(&pfad);
}
