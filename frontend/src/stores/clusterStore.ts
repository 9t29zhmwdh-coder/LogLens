import { create } from 'zustand'
import type { LogCluster, ClusterStats } from '../lib/tauri'

interface ClusterStore {
  clusters: LogCluster[]
  stats?: ClusterStats
  selected?: LogCluster
  setClusters: (c: LogCluster[]) => void
  setStats: (s: ClusterStats) => void
  select: (c: LogCluster | undefined) => void
}

export const useClusterStore = create<ClusterStore>((set) => ({
  clusters: [],
  stats: undefined,
  selected: undefined,
  setClusters: (clusters) => set({ clusters }),
  setStats: (stats) => set({ stats }),
  select: (selected) => set({ selected }),
}))
