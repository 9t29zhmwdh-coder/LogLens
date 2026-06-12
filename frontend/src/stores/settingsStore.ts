import { create } from 'zustand'
import type { AppSettings } from '../lib/tauri'

interface SettingsStore {
  settings: AppSettings
  hasKey: boolean
  setSettings: (s: AppSettings) => void
  setHasKey: (v: boolean) => void
}

const defaults: AppSettings = {
  ai_backend: 'claude',
  ollama_url: 'http://localhost:11434',
  ollama_model: 'llama3',
  theme: 'dark',
  max_entries_in_memory: 5000,
  auto_cluster: true,
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: defaults,
  hasKey: false,
  setSettings: (settings) => set({ settings }),
  setHasKey: (hasKey) => set({ hasKey }),
}))
