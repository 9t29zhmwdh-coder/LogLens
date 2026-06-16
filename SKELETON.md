# LogLens — Repository Skeleton

**Repo:** `9t29zhmwdh-coder/LogLens`
**Stack:** Rust workspace · Tauri v2 · React/TypeScript · SQLite (FTS5)
**Initial commit:** `9556dc144d8cba03440182cfb621c8bc06547efd` (2026-06-12)

---

## File Tree

```
LogLens/
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.md
│   │   └── feature_request.md
│   └── PULL_REQUEST_TEMPLATE.md
├── src-tauri/          # Rust workspace root
│   ├── ll-core/        # Core library crate
│   │   └── src/
│   │       ├── collector/    # file watcher, docker collector
│   │       ├── parser/       # JSON, plaintext, nginx parsers
│   │       ├── cluster/      # SHA2 fingerprinting, strsim similarity
│   │       ├── query/        # FTS5 full-text search
│   │       ├── ai/           # root-cause reports
│   │       └── db/           # SQLite FTS5 migrations
│   └── ll-cli/         # CLI binary crate
├── src/                # React/TypeScript frontend
├── ARCHITECTURE.md
├── CHANGELOG.md
├── CODE_OF_CONDUCT.md
├── CONTRIBUTING.md
├── PRIVACY.md
├── ROADMAP.md
├── SECURITY.md
└── SKELETON.md
```

---

## Migration Checklist

| File | Status |
|------|--------|
| SKELETON.md | ✅ pushed |
| ARCHITECTURE.md | ✅ pushed |
| CHANGELOG.md | ✅ pushed |
| CODE_OF_CONDUCT.md | ✅ pushed |
| CONTRIBUTING.md | ✅ already present |
| PRIVACY.md | ✅ pushed |
| ROADMAP.md | ✅ pushed |
| SECURITY.md | ✅ pushed |
| .github/ISSUE_TEMPLATE/bug_report.md | ✅ pushed |
| .github/ISSUE_TEMPLATE/feature_request.md | ✅ pushed |
| .github/PULL_REQUEST_TEMPLATE.md | ✅ pushed |

---

## Notes

- CI/CD workflows are not included in this skeleton (GitHub Actions requires secrets setup).
- SQLite FTS5 extension must be compiled in — verify `rusqlite` feature flags in `Cargo.toml`.
