import { useEffect, useState } from 'react'
import { api } from '../lib/tauri'
import type { LogSource } from '../lib/tauri'

export default function SourcesView() {
  const [sources, setSources] = useState<LogSource[]>([])
  const [filePath, setFilePath] = useState('')
  const [fileLabel, setFileLabel] = useState('')
  const [dockerId, setDockerId] = useState('')
  const [status, setStatus] = useState('')

  const reload = () => api.listSources().then(setSources)
  useEffect(() => { reload() }, [])

  const addFile = async () => {
    if (!filePath.trim()) return
    try {
      await api.watchFile(filePath.trim(), fileLabel.trim() || undefined)
      setFilePath(''); setFileLabel(''); setStatus('File source added')
      reload()
    } catch (e) { setStatus(String(e)) }
  }

  const addDocker = async () => {
    if (!dockerId.trim()) return
    try {
      await api.watchDocker(dockerId.trim())
      setDockerId(''); setStatus('Docker source added')
      reload()
    } catch (e) { setStatus(String(e)) }
  }

  const remove = async (id: string) => {
    await api.removeSource(id)
    reload()
  }

  return (
    <div className="p-6 max-w-2xl space-y-6">
      <h2 className="text-lg font-semibold text-ll-accent">Log Sources</h2>

      {/* Add file */}
      <div className="bg-ll-surface border border-ll-border rounded-lg p-4 space-y-3">
        <div className="text-sm font-medium text-gray-300">Add File / Directory</div>
        <input
          placeholder="/var/log/app.log  or  /var/log/*.log"
          className="w-full bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
          value={filePath} onChange={e => setFilePath(e.target.value)}
        />
        <div className="flex gap-2">
          <input
            placeholder="Label (optional)"
            className="flex-1 bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
            value={fileLabel} onChange={e => setFileLabel(e.target.value)}
          />
          <button onClick={addFile}
            className="px-4 py-1.5 bg-ll-accent/20 text-ll-accent rounded hover:bg-ll-accent/30 text-sm transition-colors">
            Watch
          </button>
        </div>
      </div>

      {/* Add Docker */}
      <div className="bg-ll-surface border border-ll-border rounded-lg p-4 space-y-3">
        <div className="text-sm font-medium text-gray-300">Add Docker Container</div>
        <div className="flex gap-2">
          <input
            placeholder="container-name or ID"
            className="flex-1 bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
            value={dockerId} onChange={e => setDockerId(e.target.value)}
          />
          <button onClick={addDocker}
            className="px-4 py-1.5 bg-ll-accent/20 text-ll-accent rounded hover:bg-ll-accent/30 text-sm transition-colors">
            Watch
          </button>
        </div>
      </div>

      {status && <div className="text-xs text-green-400">{status}</div>}

      {/* Source list */}
      <div>
        <div className="text-sm font-medium text-gray-300 mb-3">Active Sources ({sources.length})</div>
        {sources.length === 0 && <div className="text-ll-muted text-sm">No sources configured.</div>}
        {sources.map(s => (
          <div key={s.id}
            className="flex items-center justify-between py-2 border-b border-ll-border text-sm">
            <div>
              <span className="text-gray-200">{s.label}</span>
              <span className="text-ll-muted ml-2 text-xs">{JSON.stringify(s.kind)}</span>
            </div>
            <button onClick={() => remove(s.id)}
              className="text-red-400 hover:text-red-300 text-xs px-2">Remove</button>
          </div>
        ))}
      </div>
    </div>
  )
}
