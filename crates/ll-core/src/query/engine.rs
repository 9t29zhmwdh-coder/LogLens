use std::collections::HashMap;
use std::time::Instant;
use sqlx::SqlitePool;
use anyhow::Result;
use crate::models::log_entry::NormalizedEntry;
use crate::models::query::{QueryFilter, QueryRequest, QueryResult, TimelineBucket};

pub struct QueryEngine {
    pool: SqlitePool,
}

impl QueryEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn query(&self, req: &QueryRequest) -> Result<QueryResult> {
        let start = Instant::now();

        let entries = if let Some(ref text) = req.filter.text {
            self.fts_query(text, &req.filter).await?
        } else {
            self.structured_query(&req.filter).await?
        };

        let total = entries.len();
        Ok(QueryResult {
            entries,
            total,
            took_ms: start.elapsed().as_millis() as u64,
            highlights: HashMap::new(),
            ai_interpretation: None,
        })
    }

    async fn fts_query(&self, text: &str, filter: &QueryFilter) -> Result<Vec<NormalizedEntry>> {
        // Escape FTS5 special chars
        let safe = text.replace('"', "\"\"");
        let limit = filter.limit.unwrap_or(200) as i64;
        let offset = filter.offset.unwrap_or(0) as i64;

        let rows = sqlx::query!(
            r#"
            SELECT e.id, e.source_id, e.source_label, e.timestamp, e.level,
                   e.service, e.message, e.stacktrace, e.fields, e.raw,
                   e.format, e.fingerprint, e.cluster_id, e.ingested_at
            FROM log_entries e
            JOIN log_entries_fts fts ON e.rowid = fts.rowid
            WHERE log_entries_fts MATCH ?1
            ORDER BY e.timestamp DESC
            LIMIT ?2 OFFSET ?3
            "#,
            safe, limit, offset
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| row_to_entry(
            r.id, r.source_id, r.source_label, r.timestamp,
            r.level, r.service, r.message, r.stacktrace,
            r.fields, r.raw, r.format, r.fingerprint,
            r.cluster_id, r.ingested_at,
        )).collect()
    }

    async fn structured_query(&self, filter: &QueryFilter) -> Result<Vec<NormalizedEntry>> {
        let limit = filter.limit.unwrap_or(200) as i64;
        let offset = filter.offset.unwrap_or(0) as i64;

        let mut cond = vec!["1=1".to_string()];
        if let Some(ref from) = filter.from {
            cond.push(format!("timestamp >= '{}'", from.to_rfc3339()));
        }
        if let Some(ref to) = filter.to {
            cond.push(format!("timestamp <= '{}'", to.to_rfc3339()));
        }
        if let Some(ref cid) = filter.cluster_id {
            cond.push(format!("cluster_id = '{}'", cid));
        }
        if filter.has_stacktrace == Some(true) {
            cond.push("stacktrace IS NOT NULL".to_string());
        }

        let where_clause = cond.join(" AND ");
        let sql = format!(
            "SELECT id, source_id, source_label, timestamp, level, service, message, \
             stacktrace, fields, raw, format, fingerprint, cluster_id, ingested_at \
             FROM log_entries \
             WHERE {} \
             ORDER BY timestamp DESC \
             LIMIT {} OFFSET {}",
            where_clause, limit, offset
        );

        let rows = sqlx::query_as::<_, EntryRow>(&sql)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| row_to_entry(
            r.id, r.source_id, r.source_label, r.timestamp,
            r.level, r.service, r.message, r.stacktrace,
            r.fields, r.raw, r.format, r.fingerprint,
            r.cluster_id, r.ingested_at,
        )).collect()
    }

    pub async fn timeline(&self, from: &str, to: &str, bucket_minutes: u32) -> Result<Vec<TimelineBucket>> {
        // SQLite: round timestamp to nearest N-minute bucket
        let bucket_secs = bucket_minutes as i64 * 60;
        let rows = sqlx::query!(
            r#"
            SELECT
                datetime(strftime('%s', timestamp) / ?1 * ?1, 'unixepoch') AS bucket,
                level,
                COUNT(*) as cnt
            FROM log_entries
            WHERE timestamp >= ?2 AND timestamp <= ?3
            GROUP BY bucket, level
            ORDER BY bucket ASC
            "#,
            bucket_secs, from, to
        )
        .fetch_all(&self.pool)
        .await?;

        let mut map: HashMap<String, TimelineBucket> = HashMap::new();
        for row in rows {
            let bucket_str = row.bucket.unwrap_or_default();
            let entry = map.entry(bucket_str.clone()).or_insert_with(|| {
                let ts = chrono::DateTime::parse_from_rfc3339(&bucket_str)
                    .map(|t| t.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                TimelineBucket { timestamp: ts, total: 0, by_level: HashMap::new() }
            });
            let cnt = row.cnt as u64;
            entry.total += cnt;
            *entry.by_level.entry(row.level).or_insert(0) += cnt;
        }

        let mut buckets: Vec<_> = map.into_values().collect();
        buckets.sort_by_key(|b| b.timestamp);
        Ok(buckets)
    }
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    id: String, source_id: String, source_label: String,
    timestamp: String, level: String, service: Option<String>,
    message: String, stacktrace: Option<String>, fields: String,
    raw: String, format: String, fingerprint: String,
    cluster_id: Option<String>, ingested_at: String,
}

#[allow(clippy::too_many_arguments)]
fn row_to_entry(
    id: String, source_id: String, source_label: String,
    timestamp: String, level: String, service: Option<String>,
    message: String, stacktrace: Option<String>, fields: String,
    raw: String, format: String, fingerprint: String,
    cluster_id: Option<String>, ingested_at: String,
) -> Result<NormalizedEntry> {
    use crate::models::log_entry::{LogLevel, LogFormat};
    use chrono::DateTime;

    let ts = DateTime::parse_from_rfc3339(&timestamp)
        .map(|t| t.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let iat = DateTime::parse_from_rfc3339(&ingested_at)
        .map(|t| t.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let fmt = match format.as_str() {
        "json" => LogFormat::Json,
        "key_value" => LogFormat::KeyValue,
        "nginx" => LogFormat::Nginx,
        "docker" => LogFormat::Docker,
        "syslog" => LogFormat::Syslog,
        "windows_event" => LogFormat::WindowsEvent,
        _ => LogFormat::Plaintext,
    };

    let stacktrace_vec: Option<Vec<String>> = stacktrace
        .and_then(|s| serde_json::from_str(&s).ok());
    let fields_val: serde_json::Value = serde_json::from_str(&fields)
        .unwrap_or(serde_json::Value::Object(Default::default()));

    Ok(NormalizedEntry {
        id, source_id, source_label, timestamp: ts,
        level: LogLevel::from_str(&level), service,
        message, stacktrace: stacktrace_vec,
        fields: fields_val, raw, format: fmt, fingerprint,
        cluster_id, ingested_at: iat,
    })
}
