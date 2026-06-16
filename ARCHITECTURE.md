# LogLens — Architecture

## Overview

LogLens is a Rust/Tauri v2 desktop application for AI-powered log analysis. It provides real-time search, error clustering with SHA2 fingerprinting, similarity grouping, and root-cause reports. It watches both log files and Docker containers.

---

## Workspace Structure

```
src-tauri/
├── ll-core/          # Library crate — all business logic
└── ll-cli/           # Binary crate — Tauri shell + CLI entry point
```

### ll-core

| Module | Responsibility |
|--------|----------------|
| `collector/file_watcher` | Watches log files using `notify`; emits raw lines |
| `collector/docker_collector` | Polls Docker daemon via `bollard`; streams container logs |
| `parser/json_parser` | Parses structured JSON log lines (tracing, slog, Winston) |
| `parser/plaintext_parser` | Parses unstructured plaintext with heuristic level detection |
| `parser/nginx_parser` | Parses nginx access and error log formats |
| `cluster/fingerprinter` | Strips variable tokens; generates SHA2 fingerprint per log pattern |
| `cluster/similarity` | Groups fingerprints by edit distance using `strsim` |
| `query/fts5` | Full-text search over normalised entries via SQLite FTS5 |
| `ai/root_cause` | Sends cluster context to Ollama; returns root-cause report |
| `db/` | SQLite migrations; FTS5 virtual tables for entries and clusters |

### ll-cli

Tauri v2 shell: registers IPC commands, mounts the React frontend, and starts all background tasks via `tokio`.

---

## Data Flow

```
LogCollector (file/docker)
        │
        ▼
    Raw log line
        │
        ▼
    Parser (JSON / plaintext / nginx)
        │
        ▼
  Normalised LogEntry
        │
        ├──► SQLite FTS5 (full-text index)
        │
        ▼
  ClusterEngine
   ├── SHA2 fingerprint (strip tokens)
   └── strsim similarity grouping
        │
        ▼
  Cluster (pattern group + count)
        │  cluster size threshold exceeded
        ▼
  AI root-cause trigger (debounced)
        │
        └──► OllamaAnalyzer → root-cause report → SQLite
                │
                ▼
         Tauri IPC → React Frontend
```

---

## Frontend

React/TypeScript SPA served by Tauri v2. Communicates with the Rust backend exclusively via `invoke()` IPC calls.

Key views:
- **Search** — full-text search with filters (level, time range, source)
- **Clusters** — error pattern groups with occurrence counts and trend sparklines
- **Reports** — AI root-cause analysis per cluster
- **Sources** — configure watched log files and Docker targets

---

## Storage

SQLite database in the OS application data directory.

Tables: `log_entries` (FTS5 virtual table), `clusters`, `cluster_members`, `ai_reports`, `sources`, `migrations`.

The FTS5 virtual table enables substring and phrase search across millions of entries without external search infrastructure.

---

## Security

- No external network calls except `localhost:11434` (Ollama).
- All Tauri IPC commands are explicitly allowlisted in `src-tauri/capabilities/`.
- No telemetry, no crash reporting, no analytics.
