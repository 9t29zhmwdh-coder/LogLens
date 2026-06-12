use sqlx::SqlitePool;
use anyhow::Result;
use crate::models::log_entry::{NormalizedEntry, LogSource, LogSourceKind};
use crate::models::cluster::LogCluster;
use crate::models::analysis::{AiExplanation, AiSummary, RootCauseReport};

// ── Log Sources ───────────────────────────────────────────────

pub async fn insert_source(pool: &SqlitePool, src: &LogSource) -> Result<()> {
    let kind = serde_json::to_string(&src.kind)?;
    sqlx::query!(
        "INSERT OR REPLACE INTO log_sources (id, label, kind, parser_hint, enabled, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        src.id, src.label, kind, src.parser_hint,
        src.enabled, src.created_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_sources(pool: &SqlitePool) -> Result<Vec<LogSource>> {
    let rows = sqlx::query!("SELECT id, label, kind, parser_hint, enabled, created_at FROM log_sources")
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(|r| {
        let kind: LogSourceKind = serde_json::from_str(&r.kind)?;
        Ok(LogSource {
            id: r.id,
            label: r.label,
            kind,
            parser_hint: r.parser_hint,
            enabled: r.enabled != 0,
            created_at: r.created_at.parse()?,
        })
    }).collect()
}

pub async fn delete_source(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query!("DELETE FROM log_sources WHERE id = ?1", id)
        .execute(pool).await?;
    Ok(())
}

// ── Log Entries ───────────────────────────────────────────────

pub async fn insert_entry(pool: &SqlitePool, entry: &NormalizedEntry) -> Result<()> {
    let stacktrace = entry.stacktrace.as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()?;
    let fields = serde_json::to_string(&entry.fields)?;
    let format = format!("{:?}", entry.format).to_lowercase();
    let level = format!("{:?}", entry.level).to_lowercase();

    sqlx::query!(
        "INSERT OR IGNORE INTO log_entries
         (id, source_id, source_label, timestamp, level, service, message,
          stacktrace, fields, raw, format, fingerprint, cluster_id, ingested_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        entry.id, entry.source_id, entry.source_label,
        entry.timestamp, level, entry.service,
        entry.message, stacktrace, fields,
        entry.raw, format, entry.fingerprint,
        entry.cluster_id, entry.ingested_at
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count_entries(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query!("SELECT COUNT(*) as cnt FROM log_entries")
        .fetch_one(pool).await?;
    Ok(row.cnt)
}

pub async fn delete_entries_before(pool: &SqlitePool, before: &str) -> Result<u64> {
    let result = sqlx::query!("DELETE FROM log_entries WHERE timestamp < ?1", before)
        .execute(pool).await?;
    Ok(result.rows_affected())
}

// ── Clusters ─────────────────────────────────────────────────

pub async fn upsert_cluster(pool: &SqlitePool, c: &LogCluster) -> Result<()> {
    let source_ids = serde_json::to_string(&c.source_ids)?;
    let sample_ids = serde_json::to_string(&c.sample_ids)?;
    let services = serde_json::to_string(&c.services)?;
    let level = format!("{:?}", c.level).to_lowercase();

    sqlx::query!(
        "INSERT INTO log_clusters
         (id, fingerprint, template, level, count, first_seen, last_seen, source_ids, sample_ids, services, ai_summary)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(fingerprint) DO UPDATE SET
           count = excluded.count,
           last_seen = excluded.last_seen,
           source_ids = excluded.source_ids,
           sample_ids = excluded.sample_ids,
           services = excluded.services,
           ai_summary = excluded.ai_summary",
        c.id, c.fingerprint, c.template, level, c.count as i64,
        c.first_seen, c.last_seen, source_ids, sample_ids, services, c.ai_summary
    )
    .execute(pool).await?;
    Ok(())
}

pub async fn list_clusters(pool: &SqlitePool, limit: i64) -> Result<Vec<LogCluster>> {
    let rows = sqlx::query!(
        "SELECT id, fingerprint, template, level, count, first_seen, last_seen,
                source_ids, sample_ids, services, ai_summary
         FROM log_clusters ORDER BY count DESC LIMIT ?1",
        limit
    )
    .fetch_all(pool).await?;

    rows.into_iter().map(|r| {
        use crate::models::log_entry::LogLevel;
        Ok(LogCluster {
            id: r.id,
            fingerprint: r.fingerprint,
            template: r.template,
            level: LogLevel::from_str(&r.level),
            count: r.count as u64,
            first_seen: r.first_seen.parse()?,
            last_seen: r.last_seen.parse()?,
            source_ids: serde_json::from_str(&r.source_ids).unwrap_or_default(),
            sample_ids: serde_json::from_str(&r.sample_ids).unwrap_or_default(),
            services: serde_json::from_str(&r.services).unwrap_or_default(),
            ai_summary: r.ai_summary,
        })
    }).collect()
}

// ── AI Explanations ───────────────────────────────────────────

pub async fn insert_explanation(pool: &SqlitePool, e: &AiExplanation) -> Result<()> {
    let debug_steps = serde_json::to_string(&e.debug_steps)?;
    let possible_causes = serde_json::to_string(&e.possible_causes)?;
    let fix_suggestions = serde_json::to_string(&e.fix_suggestions)?;

    sqlx::query!(
        "INSERT OR REPLACE INTO ai_explanations
         (id, entry_id, created_at, what, why, impact, debug_steps, possible_causes, fix_suggestions, confidence, ai_provider, model)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        e.id, e.entry_id, e.created_at, e.what, e.why, e.impact,
        debug_steps, possible_causes, fix_suggestions,
        e.confidence, e.ai_provider, e.model
    )
    .execute(pool).await?;
    Ok(())
}

pub async fn get_explanation_for_entry(pool: &SqlitePool, entry_id: &str) -> Result<Option<AiExplanation>> {
    let row = sqlx::query!(
        "SELECT id, entry_id, created_at, what, why, impact,
                debug_steps, possible_causes, fix_suggestions, confidence, ai_provider, model
         FROM ai_explanations WHERE entry_id = ?1 ORDER BY created_at DESC LIMIT 1",
        entry_id
    )
    .fetch_optional(pool).await?;

    let Some(r) = row else { return Ok(None) };
    Ok(Some(AiExplanation {
        id: r.id, entry_id: r.entry_id, created_at: r.created_at.parse()?,
        what: r.what, why: r.why, impact: r.impact,
        debug_steps: serde_json::from_str(&r.debug_steps).unwrap_or_default(),
        possible_causes: serde_json::from_str(&r.possible_causes).unwrap_or_default(),
        fix_suggestions: serde_json::from_str(&r.fix_suggestions).unwrap_or_default(),
        confidence: r.confidence as f32, ai_provider: r.ai_provider, model: r.model,
    }))
}

// ── App Settings ──────────────────────────────────────────────

pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row = sqlx::query!("SELECT value FROM app_settings WHERE key = ?1", key)
        .fetch_optional(pool).await?;
    Ok(row.map(|r| r.value))
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query!(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        key, value
    )
    .execute(pool).await?;
    Ok(())
}
