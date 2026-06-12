<div align="center">
  <img src="RayStudio.png" alt="RayStudio Logo" width="120"/>
  <h1>LogLens</h1>
</div>

[🇩🇪 Deutsch](README.de.md)

**AI-powered log analysis · Real-time search · Error clustering · Root-cause reports**

[![Rust](https://img.shields.io/badge/Rust-1.78+-orange?logo=rust)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app)
[![License: MIT](https://img.shields.io/badge/License-MIT-green)](LICENSE)

---

## Overview

LogLens is a cross-platform developer tool that **collects, normalizes, clusters and explains logs** from any source — local files, Docker containers and system logs. It combines full-text search with AI-generated explanations (local AI via Ollama) to reduce triage time from hours to minutes.

## Features

| Module | What it does |
|---|---|
| **Multi-source collector** | Files, directories (glob), Docker containers & services, macOS Unified Logging, journald, Windows EventLog, stdin |
| **Format detection** | JSON, plaintext, key=value, Nginx combined, Docker JSON-file, syslog — auto-detected |
| **Stacktrace merging** | Multi-line stacktraces (Rust, Java, Python, JS) are automatically combined into a single entry |
| **Error clustering** | Fingerprinting strips UUIDs, IPs, timestamps → groups similar errors with similarity matching |
| **FTS5 full-text search** | SQLite FTS5 with ranked search, phrase queries and operator support |
| **AI explain** | Per-entry explanation: what happened, why, how to fix — powered by local AI (Ollama) |
| **AI block summary** | Summarize a time window: overview, key issues, root causes, recommendations |
| **Root-cause analysis** | Cluster-level deep dive: contributing factors, numbered fix steps with commands |
| **Timeline** | Stacked area chart of errors/warnings per minute — spike detection built in |
| **Export** | JSON and Markdown export |
| **CLI** | `loglens watch`, `search`, `clusters`, `analyze`, `export` |

## Quick Start

```bash
# Desktop app
cargo tauri dev

# CLI — tail a file
loglens watch /var/log/app.log --level warn

# CLI — tail Docker container + AI explain
loglens watch docker://my-api --ai

# CLI — search
loglens search "connection refused"

# CLI — show top error clusters
loglens clusters --top 20

# CLI — AI root-cause on a cluster
loglens analyze <cluster-id>

# Set API key (stored in system keychain)
loglens config set-key sk-ant-...
```

## Architecture

```
LogLens
├── crates/ll-core/          — Core library
│   ├── collector/           — File, Docker, system log collectors
│   ├── normalizer/          — Format detection + line → NormalizedEntry
│   ├── clustering/          — Fingerprinting + similarity grouper
│   ├── query/               — FTS5 query engine + AI natural-language translation
│   ├── timeline/            — Spike detection + service correlation
│   ├── ai/                  — local AI backends (Ollama) (explain / summarize / root-cause)
│   ├── export/              — JSON + Markdown export
│   └── db/                  — SQLite with FTS5 migrations
├── crates/ll-cli/           — CLI binary
├── src-tauri/               — Tauri backend + IPC commands
└── frontend/                — React + TypeScript + Recharts dashboard
```

## Tech Stack

| Layer | Technology |
|---|---|
| Core | Rust async (Tokio) |
| Desktop | Tauri v2 |
| Frontend | React 18 + TypeScript + Tailwind + Recharts |
| State | Zustand |
| Database | SQLite with FTS5 |
| File watching | notify + notify-debouncer-full |
| Docker | bollard |
| Clustering | sha2 fingerprinting + strsim similarity |
| AI | Ollama (local AI) |
| API keys | System keychain (keyring) |

## Configuration

All settings are stored in `~/.local/share/loglens/` (Linux), `~/Library/Application Support/ch.raystudio.loglens/` (macOS) or `%APPDATA%\loglens\` (Windows).

AI credentials are stored in the **system keychain** — never in plain text files.

---

<div align="right">
<sub>by</sub><br/>
<img src="RayStudio.png" alt="RayStudio" width="70"/>
</div>

**Author:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Framework Preview · **Last Updated:** June 2026
