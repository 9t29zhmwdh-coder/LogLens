import { useEffect, useState } from 'react'
import { useClusterStore } from '../stores/clusterStore'
import { api } from '../lib/tauri'
import type { LogCluster, RootCauseReport } from '../lib/tauri'

const LEVEL_BADGE: Record<string, string> = {
  error: 'bg-red-900/40 text-red-400',
  fatal: 'bg-red-800/60 text-red-300',
  warn:  'bg-yellow-900/40 text-yellow-400',
  info:  'bg-blue-900/40 text-blue-400',
  debug: 'bg-gray-800 text-gray-400',
}

export default function ClustersView() {
  const { clusters, selected, select, setClusters, setStats } = useClusterStore()
  const [report, setReport] = useState<RootCauseReport | null>(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    api.listClusters(100).then(setClusters)
    api.getClusterStats().then(setStats)
  }, [])

  const analyze = async (c: LogCluster) => {
    select(c)
    setReport(null)
    setLoading(true)
    try {
      const r = await api.analyzeCluster(c.id)
      setReport(r)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex h-full">
      <div className="flex-1 overflow-y-auto">
        <div className="px-4 py-3 border-b border-ll-border text-sm font-semibold text-ll-accent">
          Error Clusters ({clusters.length})
        </div>
        {clusters.map(c => (
          <div
            key={c.id}
            onClick={() => analyze(c)}
            className={`px-4 py-3 border-b border-ll-border cursor-pointer hover:bg-white/5 transition-colors
              ${selected?.id === c.id ? 'bg-ll-accent/10' : ''}`}
          >
            <div className="flex items-center gap-2 mb-1">
              <span className={`text-[11px] px-2 py-0.5 rounded uppercase ${LEVEL_BADGE[c.level] ?? LEVEL_BADGE.debug}`}>
                {c.level}
              </span>
              <span className="text-white font-medium text-xs truncate flex-1">{c.template}</span>
              <span className="text-ll-muted text-xs shrink-0">{c.count}×</span>
            </div>
            <div className="flex items-center gap-3 text-[11px] text-ll-muted">
              {c.services.length > 0 && <span>{c.services.slice(0, 3).join(', ')}</span>}
              <span>{new Date(c.last_seen).toLocaleString()}</span>
            </div>
          </div>
        ))}
      </div>

      {selected && (
        <div className="w-96 border-l border-ll-border bg-ll-surface flex flex-col overflow-hidden shrink-0">
          <div className="flex items-center justify-between px-3 py-2 border-b border-ll-border">
            <span className="text-xs font-semibold text-ll-accent">Root Cause Analysis</span>
            <button onClick={() => { select(undefined); setReport(null) }}
              className="text-ll-muted hover:text-gray-200 text-lg">×</button>
          </div>
          <div className="flex-1 overflow-y-auto p-3 text-xs space-y-3">
            <div>
              <div className="text-ll-muted mb-0.5">Template</div>
              <div className="text-gray-200 font-mono break-all">{selected.template}</div>
            </div>
            <div className="flex gap-4">
              <div><div className="text-ll-muted">Count</div><div className="text-white">{selected.count}</div></div>
              <div><div className="text-ll-muted">Level</div><div className="text-white uppercase">{selected.level}</div></div>
            </div>

            {loading && <div className="text-ll-muted animate-pulse">Analyzing with AI...</div>}
            {report && <ReportPanel report={report} />}
          </div>
        </div>
      )}
    </div>
  )
}

function ReportPanel({ report }: { report: RootCauseReport }) {
  return (
    <div className="space-y-3">
      <div className="text-ll-accent font-semibold">{report.title}</div>
      <div>
        <div className="text-ll-muted mb-0.5">Root Cause</div>
        <div className="text-gray-200">{report.root_cause}</div>
      </div>
      {report.contributing_factors.length > 0 && (
        <div>
          <div className="text-ll-muted mb-1">Contributing Factors</div>
          <ul className="space-y-0.5 text-yellow-300">
            {report.contributing_factors.map((f, i) => <li key={i}>• {f}</li>)}
          </ul>
        </div>
      )}
      <div>
        <div className="text-ll-muted mb-1">Fix Steps</div>
        {report.fix_suggestions.map(step => (
          <div key={step.step} className="mb-2 bg-ll-bg rounded p-2">
            <div className="text-green-400 font-medium">{step.step}. {step.title}</div>
            <div className="text-gray-300 mt-0.5">{step.description}</div>
            {step.command && (
              <code className="block mt-1 text-cyan-300 bg-black/40 rounded px-2 py-0.5">$ {step.command}</code>
            )}
          </div>
        ))}
      </div>
      <div className="text-ll-muted">Confidence: {Math.round(report.confidence * 100)}%</div>
    </div>
  )
}
