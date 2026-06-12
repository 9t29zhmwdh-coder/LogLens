CREATE TABLE IF NOT EXISTS log_clusters (
    id          TEXT PRIMARY KEY NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    template    TEXT NOT NULL,
    level       TEXT NOT NULL,
    count       INTEGER NOT NULL DEFAULT 1,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    source_ids  TEXT NOT NULL DEFAULT '[]',
    sample_ids  TEXT NOT NULL DEFAULT '[]',
    services    TEXT NOT NULL DEFAULT '[]',
    ai_summary  TEXT
);

CREATE TABLE IF NOT EXISTS ai_explanations (
    id          TEXT PRIMARY KEY NOT NULL,
    entry_id    TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    what        TEXT NOT NULL,
    why         TEXT NOT NULL,
    impact      TEXT NOT NULL,
    debug_steps TEXT NOT NULL DEFAULT '[]',
    possible_causes TEXT NOT NULL DEFAULT '[]',
    fix_suggestions TEXT NOT NULL DEFAULT '[]',
    confidence  REAL NOT NULL DEFAULT 0.0,
    ai_provider TEXT NOT NULL,
    model       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_summaries (
    id                   TEXT PRIMARY KEY NOT NULL,
    created_at           TEXT NOT NULL,
    entry_count          INTEGER NOT NULL,
    time_range_start     TEXT NOT NULL,
    time_range_end       TEXT NOT NULL,
    overview             TEXT NOT NULL,
    key_issues           TEXT NOT NULL DEFAULT '[]',
    patterns             TEXT NOT NULL DEFAULT '[]',
    root_causes          TEXT NOT NULL DEFAULT '[]',
    recommendations      TEXT NOT NULL DEFAULT '[]',
    severity_distribution TEXT NOT NULL DEFAULT '{}',
    ai_provider          TEXT NOT NULL,
    model                TEXT NOT NULL,
    tokens_used          INTEGER
);

CREATE TABLE IF NOT EXISTS root_cause_reports (
    id                   TEXT PRIMARY KEY NOT NULL,
    created_at           TEXT NOT NULL,
    trigger_entry_id     TEXT,
    cluster_id           TEXT,
    title                TEXT NOT NULL,
    root_cause           TEXT NOT NULL,
    evidence             TEXT NOT NULL DEFAULT '[]',
    contributing_factors TEXT NOT NULL DEFAULT '[]',
    fix_suggestions      TEXT NOT NULL DEFAULT '[]',
    confidence           REAL NOT NULL DEFAULT 0.0,
    ai_provider          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_clusters_fingerprint ON log_clusters (fingerprint);
CREATE INDEX IF NOT EXISTS idx_clusters_level       ON log_clusters (level, last_seen DESC);
CREATE INDEX IF NOT EXISTS idx_explanations_entry   ON ai_explanations (entry_id);
