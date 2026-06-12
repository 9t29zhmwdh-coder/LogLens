import { create } from 'zustand'
import type { NormalizedEntry, QueryFilter } from '../lib/tauri'

const MAX = 5_000

interface LogStore {
  entries: NormalizedEntry[]
  filter: QueryFilter
  selected?: NormalizedEntry
  addEntry: (e: NormalizedEntry) => void
  setFilter: (f: Partial<QueryFilter>) => void
  selectEntry: (e: NormalizedEntry | undefined) => void
  clear: () => void
}

export const useLogStore = create<LogStore>((set) => ({
  entries: [],
  filter: { limit: 200, levels: [] },
  selected: undefined,

  addEntry: (e) =>
    set((s) => ({
      entries: [e, ...s.entries].slice(0, MAX),
    })),

  setFilter: (f) =>
    set((s) => ({ filter: { ...s.filter, ...f } })),

  selectEntry: (e) => set({ selected: e }),

  clear: () => set({ entries: [], selected: undefined }),
}))
