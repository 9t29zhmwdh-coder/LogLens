# LogLens: Architecture

## Overview

LogLens is a Rust/Tauri v2 desktop application for AI-powered log analysis. It provides real-time search, error clustering with SHA2 fingerprinting, similarity grouping, and root-cause reports. It watches both log files and Docker containers.

---

## Workspace Structure

```
src-tauri/
├── ll-core/          # Library crate: all business logic
└── ll-cli/           # Binary crate: Tauri shell + CLI entry point
```

### ll-core

| Module | Responsibility |
|--------|----------------|
| `collector/file_watcher` | Watches log files using `notify`; emits raw lines |
| `collector/docker_collector` | Polls Docker daemon via `bollard`; streams container logs |
| `collector/syslog_collector` | Listens on UDP and TCP; accepts pushed lines from network appliances |
| `parser/json_parser` | Parses structured JSON log lines (tracing, slog, Winston) |
| `parser/plaintext_parser` | Parses unstructured plaintext with heuristic level detection |
| `parser/nginx_parser` | Parses nginx access and error log formats |
| `normalizer/syslog` | Parses RFC 5424 and RFC 3164 wire formats |
| `plugin/network` | Classifies parsed syslog into network events per vendor |
| `cluster/fingerprinter` | Strips variable tokens; generates SHA2 fingerprint per log pattern |
| `cluster/similarity` | Groups fingerprints by edit distance using `strsim` |
| `query/fts5` | Full-text search over normalised entries via SQLite FTS5 |
| `ai/root_cause` | Sends cluster context to Ollama; returns root-cause report |
| `ai/azure_openai` | Azure OpenAI provider, API key or Entra ID token |
| `ai/response` | Shared conversion from model JSON into the analysis types |
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

## Network log analysis

Generic log analysis treats a line as text plus a level. Network operations
need the entities inside that text: which client, on which access point,
against which firewall rule. Two layers provide that.

**The syslog layer** (`normalizer/syslog`) answers what shape a line has.
RFC 5424 is tried first because its version marker makes it unambiguous;
RFC 3164 is the fallback. Both formats are still in daily use, and a site
with mixed equipment sends both to the same port, so the decision is made
per line rather than per source.

**The classifier layer** (`plugin/network`) answers what happened. Each
vendor gets a `NetworkClassifier` with a cheap shape check and a full
extraction step. A wrong pre-shared key reaches the same
`WifiAuthFailure` event type whether it arrived as hostapd's
`WPA: invalid MIC` or as RouterOS's `rejected`, which is what makes
correlation across vendors possible.

```
Appliance (UniFi / pfSense / OPNsense / RouterOS)
        │  UDP or TCP push
        ▼
  SyslogCollector
        │
        ▼
  RFC 5424 → RFC 3164 fallback
        │
        ▼
    SyslogMessage
        │
        ▼
  ClassifierRegistry
   ├── UnifiClassifier      hostapd / dnsmasq / netfilter
   ├── PfSenseClassifier    filterlog positional CSV
   └── MikrotikClassifier   topic-prefixed RouterOS lines
        │
        ▼
    NetworkEvent  (event type + entities)
        │
        ▼
  NormalizedEntry.fields
        │
        └──► the existing FTS5 index, clustering and AI path
```

Entities land in the entry's `fields` object rather than in new columns, so
searching for a MAC address or a rule id works through the existing
full-text index with no schema change.

A line no classifier recognizes is still indexed and still searchable. Only
a line that is not syslog at all is dropped, because on a listening socket
malformed input is either a port scan or a misconfigured sender.

### Verification status of the vendor patterns

The syslog layer is tested against the examples in RFC 5424 and RFC 3164
themselves. The vendor patterns are built from published log samples and
from the upstream projects the wording originates in (hostapd, dnsmasq,
netfilter, filterlog, RouterOS). They have **not** been replayed against a
live controller's syslog stream. A pattern added here should come with a
captured sample before it is relied on in production triage.

---

## Frontend

React/TypeScript SPA served by Tauri v2. Communicates with the Rust backend exclusively via `invoke()` IPC calls.

Key views:
- **Search**: full-text search with filters (level, time range, source)
- **Clusters**: error pattern groups with occurrence counts and trend sparklines
- **Reports**: AI root-cause analysis per cluster
- **Sources**: configure watched log files, Docker targets and syslog listeners

---

## Storage

SQLite database in the OS application data directory.

Tables: `log_entries` (FTS5 virtual table), `clusters`, `cluster_members`, `ai_reports`, `sources`, `migrations`.

The FTS5 virtual table enables substring and phrase search across millions of entries without external search infrastructure.

---

## Security

- No telemetry, no crash reporting, no analytics.
- All Tauri IPC commands are explicitly allowlisted in `src-tauri/capabilities/`.

**Outbound.** With the default local backend, the only outbound call is to
`localhost:11434` (Ollama). Selecting a hosted provider sends prompt content,
which includes log lines, to that provider: `api.anthropic.com` for Claude,
or the configured resource for Azure OpenAI. The Azure endpoint is validated
against `*.openai.azure.com` and `*.cognitiveservices.azure.com` so a
mistyped host cannot quietly redirect log content elsewhere. Error bodies
from Azure are not surfaced, because they can echo the prompt.

**Inbound.** A syslog listener is the one component that accepts input from
the network. It is off unless the operator configures it, binds only where
told, and refuses privileged ports so the application never needs root.
Syslog over UDP is unauthenticated by design: anything that can reach the
port can insert lines. Bind it to a management interface, not to a public
one.

**Log content is sensitive.** Network logs carry MAC addresses, internal
addresses and host names. `MacAddr::anonymized` and
`NetworkEntities::anonymized` keep the vendor OUI and drop the device half,
for output that leaves the operator's hands.
