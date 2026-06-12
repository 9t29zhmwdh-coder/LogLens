import { useEffect, useState } from 'react'
import { api, events } from './lib/tauri'
import { useLogStore } from './stores/logStore'
import { useClusterStore } from './stores/clusterStore'
import { useSettingsStore } from './stores/settingsStore'
import LogsView from './components/LogsView'
import ClustersView from './components/ClustersView'
import TimelineView from './components/TimelineView'
import SettingsView from './components/SettingsView'
import SourcesView from './components/SourcesView'

type View = 'logs' | 'clusters' | 'timeline' | 'sources' | 'settings'

export default function App() {
  const [view, setView] = useState<View>('logs')
  const addEntry = useLogStore(s => s.addEntry)
  const setClusters = useClusterStore(s => s.setClusters)
  const { setSettings, setHasKey } = useSettingsStore()

  useEffect(() => {
    api.getSettings().then(setSettings)
    api.hasApiKey().then(setHasKey)
    api.listClusters().then(setClusters)

    const unsubs = [
      events.onLogEntry(addEntry),
      events.onClusterUpdate(setClusters),
    ]

    return () => { unsubs.forEach(p => p.then(f => f())) }
  }, [])

  const nav: { id: View; label: string; icon: string }[] = [
    { id: 'logs',     label: 'Logs',     icon: '≡' },
    { id: 'clusters', label: 'Clusters', icon: '⬡' },
    { id: 'timeline', label: 'Timeline', icon: '⌛' },
    { id: 'sources',  label: 'Sources',  icon: '⊕' },
    { id: 'settings', label: 'Settings', icon: '⚙' },
  ]

  return (
    <div className="flex h-screen overflow-hidden bg-ll-bg">
      {/* Sidebar */}
      <nav className="w-14 flex flex-col items-center py-4 gap-1 bg-ll-surface border-r border-ll-border shrink-0">
        <div className="text-ll-accent font-bold text-lg mb-4">LL</div>
        {nav.map(n => (
          <button
            key={n.id}
            title={n.label}
            onClick={() => setView(n.id)}
            className={`w-10 h-10 flex items-center justify-center rounded text-lg transition-colors
              ${view === n.id
                ? 'bg-ll-accent/20 text-ll-accent'
                : 'text-ll-muted hover:text-gray-200 hover:bg-white/5'}`}
          >
            {n.icon}
          </button>
        ))}
      </nav>

      {/* Main content */}
      <main className="flex-1 overflow-hidden">
        {view === 'logs'     && <LogsView />}
        {view === 'clusters' && <ClustersView />}
        {view === 'timeline' && <TimelineView />}
        {view === 'sources'  && <SourcesView />}
        {view === 'settings' && <SettingsView />}
      </main>
    </div>
  )
}
