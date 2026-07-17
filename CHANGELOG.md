# Changelog, LogLens

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [0.4.2] - 2026-07-17

### Security

- Bumped `vite` from `^5` to `^6` (`6.4.3`), which pulls in a patched
  `esbuild` (`^0.25.0`). Fixes 4 Dependabot alerts: esbuild dev-server
  CORS request forwarding, Vite `server.fs.deny` bypass on Windows,
  Vite path traversal in optimized-deps `.map` handling, and
  launch-editor NTLMv2 hash disclosure via UNC paths on Windows. All
  four only affect the dev server, not production builds. Verified
  `npm run build` still succeeds after the bump.

## [0.4.1] - 2026-07-17

### Changed

- README: added the missing Anthropic/Ollama line to Requirements,
  marked "(optional, for AI explain/root-cause; search, clustering and
  export work without either)".
- README.de: added the entire missing "Voraussetzungen" section (the
  German README had no requirements section at all), mirroring the
  English one.

## [0.4.0] - 2026-07-13

### Added

- Custom parser via regex template: define a named parser in Settings → Custom Parsers with a regex using named capture groups (`timestamp`, `level`, `service`, `message`, all optional), then assign it to a log source via the parser dropdown in Log Sources. A line that doesn't match falls back to auto-detection rather than being dropped. Closes the custom-parser blocker in this repo's Dual-Licensing Readiness assessment (ROADMAP.md); the multi-machine/fleet-aggregation blocker remains open by design.
- `LogSource.parser_hint` (persisted since the original schema, but never read anywhere) is now actually wired into `normalize_line`.

## [0.3.8] - 2026-07-12

### Fixed

- Removed em-dashes from ARCHITECTURE.md, CONTRIBUTING.md, and a source doc-comment in `crates/ll-core/src/models/log_entry.rs`. Swiss German orthography rule.
- Removed stale scaffold bookkeeping files `SKELETON.md` and `TEMPLATE_NOTES.md`.

## [0.3.7] - 2026-07-12

### Added

- TERMS_OF_SALE.md: terms covering the purchase of a pre-built, packaged distribution through a marketplace (as-is, no warranty, liability strictly capped at the amount paid). Does not modify the existing MIT LICENSE, which continues to cover the source code at no cost.

## [0.3.6] - 2026-07-11

### Fixed

- SemVer correction: v0.1.1 added a genuine new feature (full English/German UI translation, the app was previously English-only) but was versioned as a patch. Since a legitimate v0.2.0 minor (the cross-platform release workflow) already existed later in the history, the entire v0.2.x series was shifted to v0.3.x to make room: v0.1.1 through v0.1.3 became v0.2.0 through v0.2.2, and v0.2.0 through v0.2.5 became v0.3.0 through v0.3.5 (same commits, tags and releases recreated at identical SHAs), per the portfolio's SemVer discipline (patch = fix, minor = feature, major = finished product).

## [0.3.5] - 2026-07-11

### Added

- Documented Dual-Licensing readiness assessment in ROADMAP.md.

### Fixed

- Removed em-dashes from ROADMAP.md and SECURITY.md headings.

## [0.3.4] - 2026-07-11

### Fixed

- Updated actions/setup-node and tauri-apps/tauri-action to their latest major versions in CI and the release workflow, since GitHub is deprecating the Node.js 20 runtime and older action versions were being forced onto Node 24 and crashing during post-run cleanup.

## [0.3.3] - 2026-07-11

### Fixed

- Fixed the release workflow's stable-named DMG/installer/AppImage upload: it looked for the built bundle under `src-tauri/target/...`, but this is a Cargo workspace, so Cargo places build output under the workspace root `target/...`. The stable `LogLens.dmg`/`LogLens-Setup.exe`/`LogLens.AppImage` download links in README.md never actually got uploaded before this fix.

## [0.3.2] - 2026-07-11

### Fixed

- Corrected German README hero section: only the title image and title stay centered, tagline and badges are now left aligned like the English version

## [0.3.1] - 2026-07-10

### Fixed

- Removed em-dashes from README.md/README.de.md, replaced with colons

## [0.3.0] - 2026-07-10

### Added

- Release workflow: pushing a `v*` tag now builds macOS (DMG), Windows (NSIS installer) and Linux (deb + AppImage) bundles via `tauri-action` and attaches them to a GitHub Release. Not code-signed/notarized

## [0.2.2] - 2026-07-10

### Fixed

- `crates/ll-cli/Cargo.toml` had a hardcoded `version = "0.1.0"` instead of inheriting `version.workspace = true` like the other crates, causing it to drift out of sync with the workspace version

## [0.2.1] - 2026-07-10

### Changed

- Moved the "New here? -> beginners guide" callout in README.md above Overview (previously only appeared near Requirements)

### Added

- Added the "New here?" beginner guide callout to README.de.md (was missing)

## [0.2.0] - 2026-07-08

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
