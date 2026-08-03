# Changelog, LogLens

All notable changes to this project will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [1.2.1] - 2026-08-03

### Fixed

- Corrects a claim in the 1.2.0 entry. It said that leaving `rounded` in place would have halved every corner radius, because version 4 shifted the scale. That is wrong. Measured directly under Tailwind 4.3.3: `rounded` is still 0.25rem and is kept as an alias. The scale did shift, but under the name `rounded-sm`, which now means 0.25rem where it meant 0.125rem before. The dangerous case is source that already used `rounded-sm`, and this repository never did, so the rename changed nothing visually. The migration itself was correct; the reason given for it was not.

---

## [1.2.0] - 2026-08-03

### Changed

- Vite 6 to 8 and @vitejs/plugin-react 4 to 6. Vite 8 replaces the Rollup bundler with Rolldown. No configuration change was needed, and the output came out smaller: 620 kB of JavaScript instead of 641 kB, 11.7 kB of CSS instead of 13.1 kB.
- Tailwind CSS 3 to 4. The configuration file is gone. The custom colours and the mono font stack now live in the stylesheet as theme variables, and autoprefixer is no longer a dependency because version 4 handles prefixing itself.
- Two utility classes were renamed across five components. In version 3 `rounded` meant 0.25rem and `rounded-sm` meant half of that. Version 4 shifted the scale by one step, so the old `rounded` is now written `rounded-sm`. Keeping the old name would have quietly halved every corner radius in the interface. `outline-none` became `outline-hidden`.

### Fixed

- `.gitignore` now covers `*.db`. The database the pipeline builds to check the queries was sitting in the working directory unignored and could be committed by accident.

---

## [1.1.11] - 2026-08-02

### Added

- A smoke test in CI: the application is built, started, and checked to still be running five seconds later. Until now the pipeline only ever established that the code compiles. A program that builds cleanly and dies on launch would have passed every check and been discovered by whoever downloaded it.
- It runs on Linux and macOS. The Linux job needs `xvfb`, since a GTK window closes immediately without an X server, and that would produce a failure the runner invents rather than one the code has.
- The test also fails on a panic in the output even when the process survives, because a background task that dies quietly leaves the window open and useless.

---

## [1.1.10] - 2026-08-02

### Fixed

- **1.1.7 claimed a `bollard` upgrade that was never in it.** Its changelog describes moving to 0.21 and following the API reorganisation, but the merged pull request contained only `CHANGELOG.md`, `Cargo.toml` and `tauri.conf.json`. The version line in `crates/ll-core/Cargo.toml` and the migration in `docker_collector.rs` were missing, so the release shipped with `bollard` 0.18 and a changelog entry describing work that had not happened. The cause was mine: an intervening `git stash` removed the code changes from the working tree, and `git add -A` then committed what was left. This release contains what 1.1.7 said it did.
- The entry for 1.1.7 stays as written rather than being edited after the fact. It was published; correcting the record belongs here.

---

## [1.1.9] - 2026-08-02

### Changed

- React 18 to 19, together with `react-dom` and both type packages. Dependabot had split these across two pull requests, and neither could be merged alone: `@types/react-dom` 18 requires `@types/react` 18, so raising one of them left npm unable to resolve the peer dependency at all. Moving all four in one step resolves cleanly.
- The migration needed no code changes, which was checked rather than hoped for. `createRoot` was already in use, and the code contains no string refs, no `propTypes`, no argument-less `useRef`, no `forwardRef` and no `defaultProps`, which is the list of things React 19 removes. Callback refs, whose return value became a cleanup function in 19, do not appear either.

---

## [1.1.8] - 2026-08-02

### Fixed

- **File tailing has never worked on Windows.** `file_collector` only reacted to `Modify(Data(_))`, and Windows reports an append as `Modify(Any)`. Every appended line was therefore ignored there: no error, no message, just a window that stayed empty while the file grew. macOS and Linux send `Modify(Data(Content))`, which is why it went unnoticed. The filter accepts any `Modify` now. A surplus event costs nothing, because `read_new_lines` reads from the current position and returns immediately when there is nothing new; a missed one costs the entire feature.

### Added

- A test for the tailing loop, which had none, and a second that reports which event kinds the running platform actually sends. The second is what answered the question: guessing was not possible from a Mac, and the first attempt to reason about it reached the wrong conclusion, ruling the filter correct after checking only macOS.
- Both tests run on all three platforms. The tailing test fails on Windows without this fix, which is how the defect was established rather than assumed.

### Changed

- `notify` 6.1.1 to 8.2.0. Both versions behave identically on this point, verified by the measurement running under each: the platform decides the event kind, not the library version.
- The tailing test appends two lines rather than one, because `StacktraceAccumulator` holds a line back until the next one shows whether a continuation follows. That behaviour is now pinned.

---

## [1.1.7] - 2026-08-02

### Changed

- `bollard` 0.18.1 to 0.21.0, the Docker client. The options types moved out of the topic modules into `query_parameters` and lost their generic parameter, so `bollard::container::LogsOptions::<String>` becomes `bollard::query_parameters::LogsOptions`. `ListContainersOptions::filters` is optional now rather than a bare map.

---

## [1.1.6] - 2026-08-02

### Changed

- `sqlx` 0.8.6 to 0.9.0. The new version refuses to compile SQL that is not a `&'static str` unless the call is wrapped in `AssertSqlSafe`, which is how it forces dynamic statements to be looked at rather than waved through. That refusal is what surfaced the injection fixed in 1.1.4.
- The one dynamic statement carries that wrapper now, and the assertion is true: the string is assembled from fixed fragments only and every value arrives as a bound parameter. Before 1.1.4 the same wrapper would have been a false claim, which is why the upgrade waited for the fix rather than the other way round.
- The injection test from 1.1.4 passes unchanged under 0.9, so the guarantee survived the upgrade rather than being assumed to.

---

## [1.1.5] - 2026-08-02

### Security

- `keyring` switches from `crypto-openssl` to `crypto-rust`, which takes OpenSSL out of the Linux build. The Secret Service protocol encrypts the session between the application and the keyring daemon, and that encryption came from the OpenSSL C library, reaching this tree through `keyring` and `secret-service`. `crypto-rust` implements the same algorithms the specification prescribes, AES-CBC with SHA-2 and HKDF, from the RustCrypto crates. The wire format belongs to the specification rather than to either implementation, so an existing keyring stays readable.
- Afterwards `Cargo.lock` holds no `openssl` package at all, where it held one before. With it goes a C library with a long CVE history and the requirement to have its development headers present when building for Linux. macOS and Windows never compiled this path; both use their native keychain.

---

## [1.1.4] - 2026-08-01

### Security

- **A filter value from the frontend could rewrite the SQL query.** `structured_query` assembled its WHERE clause with `format!` and inserted `cluster_id` as a raw string between quotes. That value comes from `QueryFilter`, which the frontend fills, so an apostrophe in it escaped the literal and changed the condition. Passing `x' OR '1'='1` returned every row in the table instead of none. All values are now bound parameters and the SQL text is assembled from fixed fragments only.
- A test demonstrates it rather than asserting it: it fails against the previous code, where the crafted filter returns two rows instead of zero, and passes after the change. A second test covers the ordinary case so the fix cannot silently break normal filtering.
- Reachability, stated plainly: the frontend is this application's own code, so an attacker needs a way to influence what it sends. It is not remotely exploitable as shipped. It is still an injection in a tool whose subject is other people's log files, and the fix costs nothing.

### Fixed

- The changelog entry for 1.1.3 read "one declared dependencies ... They were" for a single item. Corrected.

---

## [1.1.3] - 2026-08-01

### Removed

- One declared dependency that no code references: `notify-debouncer-full`. It was compiled on every build, shipped its own transitive tree, counted toward the supply-chain surface, and produced a Dependabot pull request proposing an upgrade to code nobody calls. Verified by removing it and running `cargo check`, `cargo clippy` with `-D warnings` and the full test suite, all clean.

---

## [1.1.2] - 2026-08-01

### Changed

- `sha2` from 0.10 to 0.11. The new version returns a digest type that no longer implements `LowerHex`, so `format!("{:x}", ..)` stopped compiling. The hex string is now written a byte at a time. That detail matters more than it looks: the fingerprint it produces is stored in `log_clusters` in the user's database, and a different rendering would leave existing clusters pointing at nothing while new lines got fresh fingerprints, so grouping would quietly fall apart.
- A test pins the fingerprint of a fixed log line to the value measured under 0.10 before the upgrade, and a second checks the shape is lowercase hex of the expected width. Both were verified to fail when the formatting is altered, so they are evidence rather than decoration.

---

## [1.1.1] - 2026-08-01

### Changed

- Dependabot no longer retries the `glib` update it cannot perform. GHSA-wrw7-89jp-8q8g is fixed in 0.20, and this project cannot reach it: `tauri` 2.x pins `gtk ^0.18`, `gtk` 0.18 requires `glib ^0.18`, and no patched 0.18.x exists, so cargo rejects the upgrade rather than resolving it. Three attempts had already failed identically, each one a red run on `main` that carried no information. Only the unreachable versions are ignored, so a backported 0.18.x fix would still arrive, and the advisory itself stays visible in the Security tab. The block goes away when Tauri moves to gtk-rs 0.20, the condition already recorded in `SECURITY.md`.

---

## [1.1.0] - 2026-07-31

### Security

- **The default AI backend is now Ollama rather than Claude.** A fresh installation defaulted to the cloud backend, so the first explanation anyone requested sent their log lines off the machine unless they had found the setting first. Existing installations keep whatever they have stored. Requesting an explanation with Claude selected and no stored key already errored before sending, and still does.
- `SECURITY.md` claimed "no external network calls except localhost (Ollama)". That was untrue whenever the Claude backend was selected, which was the default. It now describes both backends and what each transmits.

### Added

- `SECURITY.md` records GHSA-wrw7-89jp-8q8g against `glib` 0.18.5, which cannot be fixed from this repository because Tauri 2.11.5 pins `gtk ^0.18` and no patched 0.18.x exists.

### Fixed

- The supported-versions table still listed `0.1.x`, a line that no longer exists.

---

## [1.0.10] - 2026-07-31

### Changed

- Both READMEs now open with the concrete problem clustering solves, which is a log filled with hundreds of identical stack traces from one retried request, rather than four feature nouns separated by dots. The exclusion paragraph draws the line against BugRadar explicitly: this is for a log you sit down with, BugRadar is the one that watches live.

---

## [1.0.9] - 2026-07-30

### Added

- `Cargo.lock` is committed. It was listed in `.gitignore`, so every build resolved dependencies afresh and no two builds were guaranteed to use the same versions. For an application rather than a library the lock file belongs in the repository: it is what makes a release reproducible and what lets a security advisory be checked against what actually shipped.

---

## [1.0.8] - 2026-07-30

### Changed

- The `Check` job runs on Linux, macOS and Windows instead of macOS alone. The release builds artifacts for all three, so a fault that only shows on one of the other two reached a release before anything noticed.
- The Linux leg installs the GTK and WebKit packages Tauri builds against. The runner ships neither, and without them `cargo check` fails at `gobject-2.0` before reaching any code. The release workflow already installed the same packages, which is why releases worked while no Linux check existed.
- `run_macos` in the system collector carries the same `#[allow(unused_variables, unused_mut, unreachable_code)]` its Windows counterpart already had. Both guard their body with a `cfg` block that returns early on other platforms, which leaves the rest unreachable and the parameters unused. Only the Windows function had the attribute, and with `-D warnings` the macOS one failed to compile on Linux and Windows. Nothing noticed while `Check` ran on macOS alone, where the dead path is never taken.
- The ruleset now requires `Check (ubuntu-latest)`, `Check (macos-latest)` and `Check (windows-latest)` in place of the single `Check`. A matrix renames the job, so leaving the old context required would have left a check that can never report again.

---

## [1.0.7] - 2026-07-29

### Added

- `frontend/src/vite-env.d.ts`, referencing `vite/client`. Vite has always declared modules for `*.css` and the other asset types it handles, but nothing in this project pulled that declaration in. TypeScript 5 accepts the untyped side-effect import of `index.css` regardless, so the gap stayed invisible; TypeScript 7 rejects it with `TS2882`. The file belongs to Vite's own project scaffold and was simply missing, so this closes an existing hole rather than preparing for a specific upgrade.
### Security

- The release workflow no longer grants `contents: write` for its whole run. The permission moves to the one job that publishes the release, and everything else runs with `contents: read`. OpenSSF Scorecard scores the Token-Permissions check 0 out of 10 whenever any workflow holds a top-level write permission, regardless of how little of the run needs it, so this single line was what held the check at zero.

---

## [1.0.6] - 2026-07-29

### Changed

Dependency and workflow updates merged since 1.0.5:

- chore(ci): bump the actions group across 1 directory with 3 updates
- chore(deps): bump autoprefixer

---

## [1.0.5] - 2026-07-28

### Fixed

- The CodeQL job requested `packages: read`, `actions: read` and `contents: read` at job level, repeating grants the workflow level already provides. OpenSSF Scorecard counts that as excessive token permissions and scores `Token-Permissions` at 0 out of 10 for it. The job now requests only `security-events: write`, which is the one grant that genuinely exceeds the workflow default.

## [1.0.4] - 2026-07-28

### Changed

- CodeQL moved from GitHub's default setup to an advanced setup with a committed `.github/workflows/codeql.yml`. The default setup skips pull requests that touch no code of a given language, so a dependency pull request changing only a lock file reported `skipping` on the required `Analyze (...)` checks forever and could never be merged. The workflow runs on every pull request regardless of what changed. It also uses the `security-extended` query suite, which the default setup does not allow choosing. Required checks are unchanged: verified on `BugRadar` that all eight, the generic `CodeQL` check included, turn green under this setup.
- Dependabot now groups only minor and patch updates per ecosystem; majors arrive as individual pull requests. The previous grouping put React 18 to 19, Tailwind 3 to 4 and similar breaking changes into one pull request together with urgently needed security patches, which made the whole batch unreviewable and unmergeable. Actions stay grouped wholesale. Follows `engineering-standards` v0.11.0.

## [1.0.3] - 2026-07-28

### Security

- `postcss` updated to 8.5.24, closing a high-severity path traversal in the source map auto-loading via `sourceMappingURL` that affects all versions up to and including 8.5.17.

Applied as a normal pull request rather than by merging Dependabot's, because Dependabot pull requests cannot currently pass this repository's required checks: CodeQL runs through GitHub's default setup, which does not trigger on a pull request that only touches a lock file, so its checks report `skipping` and never turn green. Bypassing a required check is not an option per `standards/ci-cd.md` section 7, so the fix takes the route that runs the full pipeline.

## [1.0.2] - 2026-07-28

### Added

- `.github/dependabot.yml`, covering GitHub Actions, the Cargo workspace and the frontend npm packages, with grouped weekly updates. The file was missing, and without it there are no version updates at all: security alerts only fire for disclosed vulnerabilities. Follows `engineering-standards` v0.10.0.

### Fixed

- `frontend/package.json` carried version 0.4.0 while the workspace and `tauri.conf.json` were on 1.0.1, the tagged version. All manifests now agree, so the next bump can touch every file that carries a version, as `release-process.md` section 2 requires.
- `actions/checkout` was pinned to two different SHAs across the workflows. All now use v7.0.1 with the full version in the comment.
- Seven actions were not pinned at all: `actions/checkout@v6`, `actions/setup-node@v6` in three places, `Swatinem/rust-cache@v2.9.1`, `tauri-apps/tauri-action@v1`, and `dtolnay/rust-toolchain@stable`, which is a branch and can be moved to point at different code at any time. All are now pinned to commit SHAs with the version in the comment, per `standards/ci-cd.md` section 2. Pinning also revealed that `Swatinem/rust-cache` was running on two different SHAs across workflows; both now use v2.9.1.
- Deliberately pinned at their current versions rather than upgraded. `actions/setup-node` would jump from v6 to v7, which is a major bump that belongs in its own reviewed PR. Dependabot, added in this same change, will now propose it.

## [1.0.1] - 2026-07-20

### Changed

- OpenSSF Scorecard workflow and badge.
- `copilot-instructions.md` for consistent AI-assisted contributions.
- Coverage reporting in CI (cargo-tarpaulin, with the sqlx database prepared and the coverage job's RUSTFLAGS relaxed so a Linux-only cfg warning doesn't fail the build).
- Unified the EN/DE language-switch link format.
- Split the README's security/CI badges onto their own line, separate from the platform/tech/AI badges (they were rendering as a single merged line).

## [1.0.0] - 2026-07-17

First stable release: a real, packaged, installable distribution exists
for end users. Real macOS/Windows/Linux installers (DMG, NSIS, AppImage/deb/rpm).

## [0.4.3] - 2026-07-17

### Changed
- CI: added an explicit `permissions: contents: read` block to the workflow(s) that were missing one (CodeQL `actions/missing-workflow-permissions`), narrowing the default GITHUB_TOKEN scope.

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
