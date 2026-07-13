# LogLens: Roadmap

## v0.1.0, Initial Release ✅ (2026-06-12)

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

## v0.2.0, Search & Cluster Improvements (planned)

- [ ] Saved search queries (named bookmarks)
- [ ] Time-range picker with relative shortcuts (last 1h, 1d, 7d)
- [ ] Cluster merge: manually group related patterns
- [ ] Export clusters and reports to JSON/Markdown
- [ ] Log level filter badges in UI
- [ ] Keyboard-first navigation (search, cluster list, report panel)

---

## v0.3.0, Parser Extensions (planned)

- [ ] Logfmt parser
- [ ] Apache access log parser
- [ ] syslog (RFC 3164 / RFC 5424) parser
- [ ] Multi-line log entry stitching (Java stack traces, Python tracebacks)
- [ ] Source tags / labels for visual grouping

---

## v0.4.0, Custom Parsers (current)

- [x] Custom parser via regex template (user-defined): named capture groups (`timestamp`, `level`, `service`, `message`, all optional), configured in Settings → Custom Parsers and assigned per source via `parser_hint`. A non-matching line falls back to auto-detection rather than being dropped.

---

## v1.0.0, Stable Release (planned)

- [ ] Full test coverage for ll-core (unit + integration)
- [ ] Signed macOS / Windows / Linux binaries
- [ ] Performance: FTS5 index over 10M entries with <100 ms query latency
- [ ] Accessibility audit (WCAG 2.1 AA)
- [ ] Comprehensive user documentation
- [ ] Automated update check (offline-first, no telemetry)

---

## Dual-Licensing Readiness

Assessed 2026-07-11 as a Dual-Licensing candidate (Community MIT + Commercial/Enterprise tier), with the same caveat as BugRadar in this portfolio: log analysis and observability is an established commercial category, but LogLens is deliberately local-first (no cloud calls except localhost Ollama, no telemetry). A conventional multi-tenant SaaS Enterprise tier would conflict with that identity. Not ready yet; blocked on:

- [ ] No multi-machine or team aggregation story at all, by design: a Commercial tier here would need to stay a licensed fleet-dashboard companion (still local/on-prem) rather than a hosted rewrite
- [x] ~~No custom parser/regex-template system yet~~ Shipped in v0.4.0 (see above): the most natural Community/Commercial split would be "core engine free, paid parser packs"
- [ ] No server or API component to gate a Commercial tier against: today this is a local desktop app plus CLI with no multi-user concept

Once the custom parser system (v0.4.0) landed, revisit: candidate Enterprise-only features would be paid parser packs and a licensed fleet-dashboard companion for aggregating multiple local LogLens instances, with the core collector/clustering/query engine, the custom parser system itself, and desktop app staying Community/MIT.
