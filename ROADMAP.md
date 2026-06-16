# LogLens — Roadmap

## v0.1.0 — Initial Release ✅ (2026-06-12)

- Real-time log file monitoring via `notify`
- Docker container log streaming via `bollard`
- JSON, plaintext, and nginx log parsers
- SHA2 fingerprint-based error clustering
- Similarity grouping via `strsim` edit distance
- AI root-cause reports via Ollama (local)
- SQLite FTS5 full-text search over log entries
- Tauri v2 desktop shell with React/TypeScript frontend
- Bilingual README (EN/DE)

---

## v0.2.0 — Search & Cluster Improvements (planned)

- [ ] Saved search queries (named bookmarks)
- [ ] Time-range picker with relative shortcuts (last 1h, 1d, 7d)
- [ ] Cluster merge: manually group related patterns
- [ ] Export clusters and reports to JSON/Markdown
- [ ] Log level filter badges in UI
- [ ] Keyboard-first navigation (search, cluster list, report panel)

---

## v0.3.0 — Parser Extensions (planned)

- [ ] Logfmt parser
- [ ] Apache access log parser
- [ ] syslog (RFC 3164 / RFC 5424) parser
- [ ] Custom parser via regex template (user-defined)
- [ ] Multi-line log entry stitching (Java stack traces, Python tracebacks)
- [ ] Source tags / labels for visual grouping

---

## v1.0.0 — Stable Release (planned)

- [ ] Full test coverage for ll-core (unit + integration)
- [ ] Signed macOS / Windows / Linux binaries
- [ ] Performance: FTS5 index over 10M entries with <100 ms query latency
- [ ] Accessibility audit (WCAG 2.1 AA)
- [ ] Comprehensive user documentation
- [ ] Automated update check (offline-first, no telemetry)
