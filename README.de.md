<div align="center">
<img src="RayStudio.png" alt="RayStudio Logo" width="120"/>

# LogLens

**KI-gestützte Log-Analyse · Echtzeit-Suche · Fehler-Clustering · Root-Cause-Berichte**

[![Rust](https://img.shields.io/badge/Rust-1.78+-orange?logo=rust)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app)
[![License: MIT](https://img.shields.io/badge/License-MIT-green)](LICENSE)

[🇬🇧 English](README.md)

</div>

---

## Übersicht

LogLens ist ein plattformübergreifendes Entwicklerwerkzeug, das **Logs aus beliebigen Quellen sammelt, normalisiert, clustert und erklärt** — lokale Dateien, Docker-Container und Systemlogs. Die Kombination aus Volltextsuche und KI-generierten Erklärungen (Claude oder Ollama) reduziert die Fehlersuche von Stunden auf Minuten.

## Funktionen

| Modul | Beschreibung |
|---|---|
| **Multi-Source-Collector** | Dateien, Verzeichnisse (Glob), Docker-Container & Services, macOS Unified Logging, journald, Windows EventLog, stdin |
| **Formaterkennung** | JSON, Plaintext, key=value, Nginx Combined, Docker JSON-File, Syslog — automatisch erkannt |
| **Stacktrace-Zusammenführung** | Mehrzeilige Stacktraces (Rust, Java, Python, JS) werden automatisch zu einem Eintrag zusammengefasst |
| **Fehler-Clustering** | Fingerprinting entfernt UUIDs, IPs, Zeitstempel → gruppiert ähnliche Fehler per Similarity-Matching |
| **FTS5-Volltextsuche** | SQLite FTS5 mit Ranking, Phrasensuche und Operatoren |
| **KI-Erklärung** | Pro Eintrag: Was ist passiert, warum, wie beheben — via Claude oder Ollama |
| **KI-Block-Zusammenfassung** | Zeitfenster zusammenfassen: Überblick, Hauptprobleme, Ursachen, Empfehlungen |
| **Root-Cause-Analyse** | Cluster-Tiefenanalyse: Einflussfaktoren, nummerierte Fix-Schritte mit Befehlen |
| **Timeline** | Gestapeltes Flächendiagramm für Fehler/Warnungen — Spike-Erkennung integriert |
| **Export** | JSON- und Markdown-Export |
| **CLI** | `loglens watch`, `search`, `clusters`, `analyze`, `export` |

## Schnellstart

```bash
# Desktop-App
cargo tauri dev

# CLI — Datei beobachten
loglens watch /var/log/app.log --level warn

# CLI — Docker-Container + KI-Erklärung
loglens watch docker://my-api --ai

# CLI — Suche
loglens search "connection refused"

# CLI — Top-Fehler-Cluster anzeigen
loglens clusters --top 20

# CLI — KI Root-Cause für einen Cluster
loglens analyze <cluster-id>

# API-Key setzen (in System-Keychain gespeichert)
loglens config set-key sk-ant-...
```

## Architektur

```
LogLens
├── crates/ll-core/          — Kernbibliothek
│   ├── collector/           — Datei-, Docker- und Systemlog-Collector
│   ├── normalizer/          — Formaterkennung + Zeile → NormalizedEntry
│   ├── clustering/          — Fingerprinting + Similarity-Grouper
│   ├── query/               — FTS5-Query-Engine + KI-Übersetzung
│   ├── timeline/            — Spike-Erkennung + Service-Korrelation
│   ├── ai/                  — Claude + Ollama (erklären / zusammenfassen / Root-Cause)
│   ├── export/              — JSON + Markdown Export
│   └── db/                  — SQLite mit FTS5-Migrationen
├── crates/ll-cli/           — CLI-Binary
├── src-tauri/               — Tauri-Backend + IPC-Commands
└── frontend/                — React + TypeScript + Recharts Dashboard
```

## Tech-Stack

| Schicht | Technologie |
|---|---|
| Core | Rust async (Tokio) |
| Desktop | Tauri v2 |
| Frontend | React 18 + TypeScript + Tailwind + Recharts |
| State | Zustand |
| Datenbank | SQLite mit FTS5 |
| Datei-Watching | notify + notify-debouncer-full |
| Docker | bollard |
| Clustering | sha2-Fingerprinting + strsim-Similarity |
| KI | Claude (`claude-haiku-4-5`) + Ollama |
| API-Keys | System-Keychain (keyring) |

## Konfiguration

Alle Einstellungen werden gespeichert unter `~/Library/Application Support/ch.raystudio.loglens/` (macOS), `~/.local/share/loglens/` (Linux) oder `%APPDATA%\loglens\` (Windows).

Der Claude-API-Key wird ausschliesslich im **System-Keychain** gespeichert — niemals als Klartext.

---

<div align="right">
<sub>by</sub><br/>
<img src="RayStudio.png" alt="RayStudio" width="70"/>
</div>

**Author:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Framework Preview · **Last Updated:** Juni 2026
