import { useState } from 'react'
import { useSettingsStore } from '../stores/settingsStore'
import { api } from '../lib/tauri'

export default function SettingsView() {
  const { settings, setSettings, hasKey, setHasKey } = useSettingsStore()
  const [apiKey, setApiKey] = useState('')
  const [status, setStatus] = useState('')
  const [testing, setTesting] = useState(false)

  const saveKey = async () => {
    if (!apiKey.trim()) return
    await api.saveApiKey(apiKey.trim())
    setApiKey('')
    setHasKey(true)
    setStatus('API key saved')
  }

  const testBackend = async () => {
    setTesting(true)
    const ok = await api.checkAiBackend()
    setStatus(ok ? 'AI backend reachable ✓' : 'AI backend not reachable ✗')
    setTesting(false)
  }

  const save = async () => {
    await api.saveSettings(settings)
    setStatus('Settings saved')
  }

  return (
    <div className="p-6 max-w-xl space-y-6">
      <h2 className="text-lg font-semibold text-ll-accent">Settings</h2>

      {/* AI Backend */}
      <section className="bg-ll-surface border border-ll-border rounded-lg p-4 space-y-4">
        <div className="text-sm font-semibold text-gray-300">AI Backend</div>
        <div className="flex gap-3">
          {['claude', 'ollama'].map(b => (
            <button
              key={b}
              onClick={() => setSettings({ ...settings, ai_backend: b })}
              className={`px-4 py-1.5 rounded text-sm transition-colors capitalize
                ${settings.ai_backend === b
                  ? 'bg-ll-accent text-ll-bg'
                  : 'border border-ll-border text-ll-muted hover:text-gray-200'}`}
            >
              {b}
            </button>
          ))}
        </div>

        {settings.ai_backend === 'claude' && (
          <div className="space-y-2">
            <div className="text-xs text-ll-muted">
              Claude API Key {hasKey ? '· ✓ Set' : '· Not configured'}
            </div>
            <div className="flex gap-2">
              <input
                type="password"
                placeholder="sk-ant-..."
                className="flex-1 bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
                value={apiKey}
                onChange={e => setApiKey(e.target.value)}
              />
              <button onClick={saveKey}
                className="px-4 py-1.5 bg-ll-accent/20 text-ll-accent rounded hover:bg-ll-accent/30 text-sm transition-colors">
                Save
              </button>
            </div>
          </div>
        )}

        {settings.ai_backend === 'ollama' && (
          <div className="space-y-2">
            <input
              placeholder="Ollama URL"
              className="w-full bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
              value={settings.ollama_url}
              onChange={e => setSettings({ ...settings, ollama_url: e.target.value })}
            />
            <input
              placeholder="Model (e.g. llama3)"
              className="w-full bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 placeholder-ll-muted focus:outline-none focus:border-ll-accent"
              value={settings.ollama_model}
              onChange={e => setSettings({ ...settings, ollama_model: e.target.value })}
            />
          </div>
        )}

        <button onClick={testBackend} disabled={testing}
          className="text-xs text-ll-muted hover:text-gray-200 transition-colors">
          {testing ? 'Testing...' : 'Test connection'}
        </button>
      </section>

      {/* General */}
      <section className="bg-ll-surface border border-ll-border rounded-lg p-4 space-y-4">
        <div className="text-sm font-semibold text-gray-300">General</div>
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={settings.auto_cluster}
            onChange={e => setSettings({ ...settings, auto_cluster: e.target.checked })}
            className="w-4 h-4 accent-ll-accent"
          />
          <span className="text-sm text-gray-200">Auto-cluster similar errors</span>
        </label>
        <div>
          <div className="text-xs text-ll-muted mb-1">Max entries in memory</div>
          <input
            type="number"
            min={1000} max={50000} step={1000}
            className="bg-ll-bg border border-ll-border rounded px-3 py-1.5 text-sm text-gray-200 w-32 focus:outline-none focus:border-ll-accent"
            value={settings.max_entries_in_memory}
            onChange={e => setSettings({ ...settings, max_entries_in_memory: parseInt(e.target.value) })}
          />
        </div>
      </section>

      <div className="flex items-center gap-4">
        <button onClick={save}
          className="px-5 py-2 bg-ll-accent text-ll-bg rounded font-medium text-sm hover:opacity-90 transition-opacity">
          Save Settings
        </button>
        {status && <span className="text-xs text-green-400">{status}</span>}
      </div>
    </div>
  )
}
