CREATE TABLE IF NOT EXISTS log_sources (
    id          TEXT PRIMARY KEY NOT NULL,
    label       TEXT NOT NULL,
    kind        TEXT NOT NULL,
    parser_hint TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS log_entries (
    id           TEXT PRIMARY KEY NOT NULL,
    source_id    TEXT NOT NULL,
    source_label TEXT NOT NULL,
    timestamp    TEXT NOT NULL,
    level        TEXT NOT NULL,
    service      TEXT,
    message      TEXT NOT NULL,
    stacktrace   TEXT,
    fields       TEXT NOT NULL DEFAULT '{}',
    raw          TEXT NOT NULL DEFAULT '',
    format       TEXT NOT NULL DEFAULT 'unknown',
    fingerprint  TEXT NOT NULL DEFAULT '',
    cluster_id   TEXT,
    ingested_at  TEXT NOT NULL
);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS log_entries_fts USING fts5(
    id UNINDEXED,
    message,
    service,
    raw,
    content='log_entries',
    content_rowid='rowid'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS log_entries_ai AFTER INSERT ON log_entries BEGIN
    INSERT INTO log_entries_fts(rowid, id, message, service, raw)
    VALUES (new.rowid, new.id, new.message, COALESCE(new.service,''), new.raw);
END;

CREATE TRIGGER IF NOT EXISTS log_entries_ad AFTER DELETE ON log_entries BEGIN
    INSERT INTO log_entries_fts(log_entries_fts, rowid, id, message, service, raw)
    VALUES ('delete', old.rowid, old.id, old.message, COALESCE(old.service,''), old.raw);
END;

CREATE INDEX IF NOT EXISTS idx_entries_source_time  ON log_entries (source_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_entries_level        ON log_entries (level, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_entries_fingerprint  ON log_entries (fingerprint);
CREATE INDEX IF NOT EXISTS idx_entries_cluster      ON log_entries (cluster_id);
CREATE INDEX IF NOT EXISTS idx_entries_service      ON log_entries (service, timestamp DESC);
