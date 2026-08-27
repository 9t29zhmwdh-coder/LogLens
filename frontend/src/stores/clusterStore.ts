import { create } from 'zustand'
import type { LogCluster, ClusterStats } from '../lib/tauri'

interface ClusterStore {
  clusters: LogCluster[]
  stats?: ClusterStats
  selected?: LogCluster
  setClusters: (c: LogCluster[]) => void
  /** Live updates carry only the largest few clusters. Replacing the list
   *  with them dropped everything else, which is why the view emptied itself
   *  on every incoming log line. */
  mergeClusters: (c: LogCluster[]) => void
  setStats: (s: ClusterStats) => void
  select: (c: LogCluster | undefined) => void
}

export const useClusterStore = create<ClusterStore>((set) => ({
  clusters: [],
  stats: undefined,
  selected: undefined,
  setClusters: (clusters) => set({ clusters }),
  mergeClusters: (incoming) => set((state) => {
    const byId = new Map(state.clusters.map(c => [c.id, c]))
    for (const c of incoming) byId.set(c.id, c)
    return { clusters: [...byId.values()].sort((a, b) => b.count - a.count) }
  }),
  setStats: (stats) => set({ stats }),
  select: (selected) => set({ selected }),
}))
