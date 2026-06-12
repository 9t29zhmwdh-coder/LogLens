import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// ── Types ──────────────────────────────────────────────────────

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error' | 'fatal' | 'unknown'

export interface NormalizedEntry {
  id: string
  source_id: string
  source_label: string
  timestamp: string
  level: LogLevel
  service?: string
  message: string
  stacktrace?: string[]
  fields: Record<string, unknown>
  raw: string
  format: string
  fingerprint: string
  cluster_id?: string
  ingested_at: string
}

export interface LogSource {
  id: string
  label: string
  kind: LogSourceKind
  parser_hint?: string
  enabled: boolean
  created_at: string
}

export type LogSourceKind =
  | { File: { path: string } }
  | { Directory: { path: string; pattern?: string } }
  | { DockerContainer: { container_id: string; name: string } }
  | { DockerService: { service_name: string } }
  | 'Stdin' | 'SystemMacos' | 'Journald'
  | { WindowsEventLog: { channel: string } }

export interface QueryFilter {
  text?: string
  levels?: LogLevel[]
  services?: string[]
  source_ids?: string[]
  from?: string
  to?: string
  has_stacktrace?: boolean
  cluster_id?: string
  limit?: number
  offset?: number
}

export interface QueryRequest {
  filter: QueryFilter
  ai_query?: string
  highlight: boolean
  sort_desc: boolean
}

export interface QueryResult {
  entries: NormalizedEntry[]
  total: number
  took_ms: number
  highlights: Record<string, string[]>
  ai_interpretation?: string
}

export interface TimelineBucket {
  timestamp: string
  total: number
  by_level: Record<string, number>
}

export interface LogCluster {
  id: string
  fingerprint: string
  template: string
  level: LogLevel
  count: number
  first_seen: string
  last_seen: string
  source_ids: string[]
  sample_ids: string[]
  services: string[]
  ai_summary?: string
}

export interface ClusterStats {
  total_clusters: number
  total_entries: number
  top_errors: LogCluster[]
  by_level: Record<string, number>
}

export interface AiExplanation {
  id: string
  entry_id: string
  created_at: string
  what: string
  why: string
  impact: string
  debug_steps: string[]
  possible_causes: string[]
  fix_suggestions: string[]
  confidence: number
  ai_provider: string
  model: string
}

export interface AiSummary {
  id: string
  overview: string
  key_issues: string[]
  patterns: string[]
  root_causes: string[]
  recommendations: string[]
  severity_distribution: Record<string, number>
  entry_count: number
}

export interface RootCauseReport {
  id: string
  title: string
  root_cause: string
  evidence: string[]
  contributing_factors: string[]
  fix_suggestions: FixStep[]
  confidence: number
  ai_provider: string
}

export interface FixStep {
  step: number
  title: string
  description: string
  command?: string
  code?: string
}

export interface AppSettings {
  ai_backend: string
  ollama_url: string
  ollama_model: string
  theme: string
  max_entries_in_memory: number
  auto_cluster: boolean
}

// ── Commands ───────────────────────────────────────────────────

export const api = {
  // Settings
  getSettings: () => invoke<AppSettings>('get_settings'),
  saveSettings: (s: AppSettings) => invoke<void>('save_settings', { settings: s }),
  saveApiKey: (key: string) => invoke<void>('save_api_key', { key }),
  hasApiKey: () => invoke<boolean>('has_api_key'),
  checkAiBackend: () => invoke<boolean>('check_ai_backend'),

  // Collector
  watchFile: (path: string, label?: string) => invoke<string>('watch_file', { path, label }),
  watchDocker: (containerId: string, name?: string) => invoke<string>('watch_docker', { containerId, name }),
  removeSource: (sourceId: string) => invoke<void>('remove_source', { sourceId }),
  listSources: () => invoke<LogSource[]>('list_sources'),

  // Query
  queryLogs: (req: QueryRequest) => invoke<QueryResult>('query_logs', { req }),
  getTimeline: (from: string, to: string, bucketMinutes?: number) =>
    invoke<TimelineBucket[]>('get_timeline', { from, to, bucketMinutes }),
  translateQuery: (nlQuery: string) => invoke<QueryFilter>('translate_query', { nlQuery }),

  // Clusters
  listClusters: (limit?: number) => invoke<LogCluster[]>('list_clusters', { limit }),
  getClusterStats: () => invoke<ClusterStats>('get_cluster_stats'),

  // AI
  explainEntry: (entry: NormalizedEntry) => invoke<AiExplanation>('explain_entry', { entry }),
  getCachedExplanation: (entryId: string) => invoke<AiExplanation | null>('get_cached_explanation', { entryId }),
  summarizeEntries: (entries: NormalizedEntry[]) => invoke<AiSummary>('summarize_entries', { entries }),
  analyzeCluster: (clusterId: string) => invoke<RootCauseReport>('analyze_cluster', { clusterId }),
}

// ── Events ─────────────────────────────────────────────────────

export const events = {
  onLogEntry: (cb: (e: NormalizedEntry) => void) => listen<NormalizedEntry>('log://entry', ev => cb(ev.payload)),
  onClusterUpdate: (cb: (clusters: LogCluster[]) => void) =>
    listen<LogCluster[]>('cluster://updated', ev => cb(ev.payload)),
}
