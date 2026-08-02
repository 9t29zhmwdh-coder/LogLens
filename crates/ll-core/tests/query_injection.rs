//! Weist nach, dass ein Filterwert aus dem Frontend kein SQL mehr aendern kann.
//!
//! `structured_query` baute die WHERE-Klausel per `format!` zusammen und setzte
//! `cluster_id` als rohen String in Anfuehrungszeichen ein. Der Wert stammt aus
//! `QueryFilter`, den das Frontend fuellt, ein Apostroph darin brach also aus
//! dem Literal aus.
//!
//! Der erste Test schlaegt gegen die alte Fassung fehl: dort liefert der
//! praeparierte Filter alle Zeilen statt keiner.

use ll_core::models::query::{QueryFilter, QueryRequest};
use ll_core::query::engine::QueryEngine;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn pool_mit_zwei_zeilen() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("In-Memory-Datenbank");

    sqlx::query(
        r#"CREATE TABLE log_entries (
            id TEXT PRIMARY KEY, source_id TEXT, source_label TEXT, timestamp TEXT,
            level TEXT, service TEXT, message TEXT, stacktrace TEXT, fields TEXT,
            raw TEXT, format TEXT, fingerprint TEXT, cluster_id TEXT, ingested_at TEXT
        )"#,
    )
    .execute(&pool)
    .await
    .expect("Tabelle");

    for (id, cluster) in [("a", "cluster-eins"), ("b", "cluster-zwei")] {
        sqlx::query(
            "INSERT INTO log_entries (id, source_id, source_label, timestamp, level, service,
             message, stacktrace, fields, raw, format, fingerprint, cluster_id, ingested_at)
             VALUES (?, 's', 'S', '2026-01-01T00:00:00Z', 'ERROR', 'svc', 'm', NULL, '{}', 'r',
             'plain', 'fp', ?, '2026-01-01T00:00:00Z')",
        )
        .bind(id)
        .bind(cluster)
        .execute(&pool)
        .await
        .expect("Zeile");
    }
    pool
}

fn filter_mit_cluster(cluster_id: &str) -> QueryRequest {
    QueryRequest {
        filter: QueryFilter {
            cluster_id: Some(cluster_id.to_string()),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn a_quote_in_the_cluster_filter_cannot_widen_the_query() {
    let engine = QueryEngine::new(pool_mit_zwei_zeilen().await);

    // Bricht der Wert aus dem Literal aus, wird die Bedingung wahr und die
    // Abfrage liefert beide Zeilen. Gebunden liefert sie keine, weil kein
    // Cluster so heisst.
    let bösartig = "x' OR '1'='1";
    let r = engine.query(&filter_mit_cluster(bösartig)).await.expect("Abfrage");

    assert_eq!(
        r.entries.len(),
        0,
        "der Filterwert hat die WHERE-Klausel veraendert statt nur verglichen zu werden"
    );
}

#[tokio::test]
async fn an_ordinary_cluster_filter_still_matches() {
    let engine = QueryEngine::new(pool_mit_zwei_zeilen().await);
    let r = engine.query(&filter_mit_cluster("cluster-eins")).await.expect("Abfrage");
    assert_eq!(r.entries.len(), 1, "der gewoehnliche Fall muss weiter funktionieren");
}
