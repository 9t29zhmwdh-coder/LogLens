import { useState, useRef, useEffect } from 'react'
import { useLogStore } from '../stores/logStore'
import { api } from '../lib/tauri'
import type { NormalizedEntry, AiExplanation } from '../lib/tauri'

const LEVEL_COLORS: Record<string, string> = {
  trace: 'text-gray-500', debug: 'text-gray-400', info: 'text-blue-400',
  warn: 'text-yellow-500', error: 'text-red-400', fatal: 'text-red-300 font-semibold',
  unknown: 'text-gray-400',
}

export default function LogsView() {
  const { entries, filter, setFilter, selected, selectEntry } = useLogStore()
  const [searchInput, setSearchInput] = useState('')
  const [aiExpl, setAiExpl] = useState<AiExplanation | null>(null)
  const [loadingAi, setLoadingAi] = useState(false)
  const bottomRef = useRef<HTMLDivElement>(null)

  const filtered = entries.filter(e => {
    if (filter.levels?.length && !filter.levels.includes(e.level)) return false
    if (searchInput && !e.message.toLowerCase().includes(searchInput.toLowerCase())) return false
    return true
  })

  const handleExplain = async (entry: NormalizedEntry) => {
    selectEntry(entry)
    setAiExpl(null)
    setLoadingAi(true)
    try {
      const cached = await api.getCachedExplanation(entry.id)
      if (cached) { setAiExpl(cached); return }
      const expl = await api.explainEntry(entry)
      setAiExpl(expl)
    } finally {
      setLoadingAi(false)
    }
  }

  return (
    <div className="flex h-full">
      {/* Log list */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Toolbar */}
        <div className="flex items-center gap-2 px-3 py-2 border-b border-ll-border bg-ll-surface shrink-0">
          <input
            type="text"
            placeholder="Search logs..."
            className="flex-1 bg-ll-bg border border-ll-border rounded px-3 py-1 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
            value={searchInput}
            onChange={e => setSearchInput(e.target.value)}
          />
          <LevelFilter />
          <span className="text-xs text-ll-muted ml-2">{filtered.length} entries</span>
        </div>

        {/* Entries */}
        <div className="flex-1 overflow-y-auto font-mono text-xs">
          {filtered.map(entry => (
            <LogLine
              key={entry.id}
              entry={entry}
              selected={selected?.id === entry.id}
              onClick={() => handleExplain(entry)}
            />
          ))}
          <div ref={bottomRef} />
        </div>
      </div>

      {/* Detail panel */}
      {selected && (
        <div className="w-96 border-l border-ll-border bg-ll-surface flex flex-col overflow-hidden shrink-0">
          <div className="flex items-center justify-between px-3 py-2 border-b border-ll-border">
            <span className="text-xs font-semibold text-ll-accent">Entry Detail</span>
            <button onClick={() => { selectEntry(undefined); setAiExpl(null) }}
              className="text-ll-muted hover:text-gray-200 text-lg">×</button>
          </div>
          <div className="flex-1 overflow-y-auto p-3 space-y-3 text-xs">
            <Field label="Timestamp" value={selected.timestamp} />
            <Field label="Level" value={selected.level} />
            <Field label="Service" value={selected.service ?? '-'} />
            <Field label="Source" value={selected.source_label} />
            <Field label="Message" value={selected.message} multiline />
            {selected.stacktrace && (
              <div>
                <div className="text-ll-muted mb-1">Stacktrace</div>
                <pre className="text-red-300 text-[11px] whitespace-pre-wrap break-all bg-ll-bg rounded p-2">
                  {selected.stacktrace.join('\n')}
                </pre>
              </div>
            )}

            {/* AI explanation */}
            <div className="border-t border-ll-border pt-3">
              {loadingAi && <div className="text-ll-muted animate-pulse">AI analyzing...</div>}
              {aiExpl && <AiPanel expl={aiExpl} />}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

function LogLine({ entry, selected, onClick }: {
  entry: NormalizedEntry; selected: boolean; onClick: () => void
}) {
  return (
    <div
      onClick={onClick}
      className={`flex items-start gap-2 px-3 py-0.5 cursor-pointer hover:bg-white/5 border-b border-ll-border/30
        ${selected ? 'bg-ll-accent/10' : ''}`}
    >
      <span className="text-ll-muted shrink-0 w-20">
        {new Date(entry.timestamp).toLocaleTimeString('en', { hour12: false, fractionalSecondDigits: 3 })}
      </span>
      <span className={`shrink-0 w-12 uppercase ${LEVEL_COLORS[entry.level] ?? ''}`}>
        {entry.level}
      </span>
      {entry.service && (
        <span className="text-purple-400 shrink-0 max-w-[80px] truncate">[{entry.service}]</span>
      )}
      <span className="text-gray-200 truncate flex-1">{entry.message}</span>
      {entry.stacktrace && <span className="text-red-400 shrink-0 text-[10px]">⚠ trace</span>}
    </div>
  )
}

function LevelFilter() {
  const { filter, setFilter } = useLogStore()
  const levels = ['debug', 'info', 'warn', 'error', 'fatal'] as const

  const toggle = (level: string) => {
    const current = filter.levels ?? []
    const next = current.includes(level as never)
      ? current.filter(l => l !== level)
      : [...current, level as never]
    setFilter({ levels: next })
  }

  return (
    <div className="flex gap-1">
      {levels.map(l => (
        <button
          key={l}
          onClick={() => toggle(l)}
          className={`px-2 py-0.5 rounded text-[11px] uppercase transition-colors
            ${(filter.levels ?? []).includes(l as never)
              ? `${LEVEL_COLORS[l]} bg-white/10`
              : 'text-ll-muted hover:text-gray-300'}`}
        >
          {l}
        </button>
      ))}
    </div>
  )
}

function Field({ label, value, multiline }: { label: string; value: string; multiline?: boolean }) {
  return (
    <div>
      <div className="text-ll-muted mb-0.5">{label}</div>
      {multiline
        ? <div className="text-gray-200 break-all whitespace-pre-wrap bg-ll-bg rounded p-2">{value}</div>
        : <div className="text-gray-200">{value}</div>
      }
    </div>
  )
}

function AiPanel({ expl }: { expl: AiExplanation }) {
  return (
    <div className="space-y-2">
      <div className="text-ll-accent text-[11px] font-semibold">
        AI Analysis · {Math.round(expl.confidence * 100)}% confidence
      </div>
      <div>
        <span className="text-ll-muted">What: </span>
        <span className="text-gray-200">{expl.what}</span>
      </div>
      <div>
        <span className="text-ll-muted">Why: </span>
        <span className="text-gray-200">{expl.why}</span>
      </div>
      {expl.fix_suggestions.length > 0 && (
        <div>
          <div className="text-ll-muted mb-1">Fix suggestions</div>
          <ul className="space-y-0.5">
            {expl.fix_suggestions.map((s, i) => (
              <li key={i} className="text-green-400">→ {s}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
