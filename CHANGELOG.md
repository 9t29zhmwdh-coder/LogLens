# Changelog — LogLens

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.0] — 2026-06-12

### Added

- Real-time log file monitoring using `notify` (cross-platform)
- Docker container log streaming via `bollard`
- Log parsers: JSON (tracing/slog/Winston), plaintext, nginx access/error
- SHA2 fingerprint-based error clustering (strips variable tokens)
- Similarity grouping via `strsim` edit distance for related patterns
- AI root-cause reports per cluster via Ollama (`localhost:11434`)
- SQLite FTS5 full-text search over normalised log entries
- Tauri v2 desktop shell with React/TypeScript frontend
- Search view with level and time-range filters
- Cluster view with occurrence counts and trend sparklines
- AI report panel per cluster
- Source configuration UI (log files + Docker targets)
- Bilingual README (English / German)
- CONTRIBUTING.md with development setup guide
