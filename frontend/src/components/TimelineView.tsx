import { useEffect, useState } from 'react'
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts'
import { api } from '../lib/tauri'
import type { TimelineBucket } from '../lib/tauri'
import { subHours, format } from 'date-fns'

export default function TimelineView() {
  const [buckets, setBuckets] = useState<TimelineBucket[]>([])
  const [range, setRange] = useState(6)

  useEffect(() => {
    const to = new Date().toISOString()
    const from = subHours(new Date(), range).toISOString()
    api.getTimeline(from, to, 5).then(setBuckets)
  }, [range])

  const data = buckets.map(b => ({
    time: format(new Date(b.timestamp), 'HH:mm'),
    error: b.by_level['error'] ?? 0,
    warn:  b.by_level['warn'] ?? 0,
    info:  b.by_level['info'] ?? 0,
    total: b.total,
  }))

  return (
    <div className="p-6 h-full overflow-y-auto">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-lg font-semibold text-ll-accent">Log Timeline</h2>
        <div className="flex gap-2">
          {[1, 6, 24, 72].map(h => (
            <button
              key={h}
              onClick={() => setRange(h)}
              className={`px-3 py-1 rounded text-xs transition-colors
                ${range === h ? 'bg-ll-accent text-ll-bg' : 'border border-ll-border text-ll-muted hover:text-gray-200'}`}
            >
              {h}h
            </button>
          ))}
        </div>
      </div>

      <div className="bg-ll-surface border border-ll-border rounded-lg p-4 mb-6">
        <div className="text-xs text-ll-muted mb-3">Errors & Warnings per 5 min</div>
        <ResponsiveContainer width="100%" height={240}>
          <AreaChart data={data}>
            <CartesianGrid strokeDasharray="3 3" stroke="#30363d" />
            <XAxis dataKey="time" stroke="#8b949e" tick={{ fontSize: 11 }} />
            <YAxis stroke="#8b949e" tick={{ fontSize: 11 }} />
            <Tooltip
              contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6 }}
              labelStyle={{ color: '#e6edf3' }}
            />
            <Area type="monotone" dataKey="error" stackId="1" stroke="#f85149" fill="#f85149" fillOpacity={0.3} />
            <Area type="monotone" dataKey="warn" stackId="1" stroke="#d29922" fill="#d29922" fillOpacity={0.3} />
            <Area type="monotone" dataKey="info" stackId="1" stroke="#58a6ff" fill="#58a6ff" fillOpacity={0.2} />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}
