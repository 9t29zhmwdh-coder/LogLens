# Changelog, LogLens

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.1.2] - 2026-07-10

### Changed

- Moved the "New here? -> beginners guide" callout in README.md above Overview (previously only appeared near Requirements)

### Added

- Added the "New here?" beginner guide callout to README.de.md (was missing)

## [0.1.1] - 2026-07-08

### Fixed

- App crashed on every launch: `.setup()` called `tokio::runtime::Handle::current()`,
  which panics ("there is no reactor running") since no ambient tokio runtime
  exists in that context. Switched to `tauri::async_runtime::block_on`/`spawn`,
  the correct Tauri v2 pattern
- `beforeDevCommand`/`beforeBuildCommand` in `tauri.conf.json` used `cd frontend`
  instead of `cd ../frontend`; `cargo tauri dev`/`build` failed for anyone
  following the README's Quick Start exactly as written
- Missing `thiserror` and `sqlx` dependencies in `src-tauri/Cargo.toml`; the app
  crate failed to compile at all. Promoted `sqlx` to a workspace dependency
  shared with `ll-core`
- A local `Result<T>` alias was shadowing `std::result::Result` in the
  `Serialize` impl for `LlError`
- Missing `src-tauri/capabilities/` permissions were blocking the event system
- Icons referenced in `tauri.conf.json` did not exist in the repo
- Unused `tauri-plugin-shell` dependency (registered but never invoked)
- Unused `AppState.log_tx` field and an unnecessary `mut` binding (clippy)
- `@import` for the Google Font came after `@tailwind` directives in `index.css`,
  which is invalid CSS order
- Missing `ES2021.Intl` lib entry in `tsconfig.json` broke the TypeScript build
  (`fractionalSecondDigits` was unrecognized); no frontend CI job existed to
  catch this before
- CI excluded the `loglens-tauri` crate from all checks, hiding all of the above
- README claimed the AI backend was Ollama-only; the app actually supports both
  Claude (default) and Ollama. Corrected badges, feature descriptions and
  architecture notes in both READMEs

### Added

- Full English/German UI translation (the app was previously English-only
  with no language toggle)
- README onboarding sections: how it runs, screenshot, in practice, uninstall/cleanup

## [0.1.0] - 2026-06-12

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
