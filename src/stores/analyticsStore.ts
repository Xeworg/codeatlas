// analyticsStore.ts — v2 analytical views and filter state
// Part of PR5: UX analítica + filtros persistentes

import { create } from 'zustand'

export type AnalyticalView = 'architecture' | 'dependencies' | 'flow-beta'

interface AnalyticsState {
  // Active analytical view
  activeView: AnalyticalView

  // Filters
  nodeTypeFilter: string | null
  couplingThreshold: number
  showCycles: boolean
  showHotspots: boolean

  // Actions
  setView: (view: AnalyticalView) => void
  setFilter: <K extends keyof AnalyticsState>(key: K, value: AnalyticsState[K]) => void
  resetFilters: () => void
  resetAnalytics: () => void
}

const DEFAULT_ANALYTICS_STATE: Pick<
  AnalyticsState,
  'activeView' | 'nodeTypeFilter' | 'couplingThreshold' | 'showCycles' | 'showHotspots'
> = {
  activeView: 'architecture',
  nodeTypeFilter: null,
  couplingThreshold: 0.5,
  showCycles: false,
  showHotspots: false,
}

export const useAnalyticsStore = create<AnalyticsState>((set) => ({
  ...DEFAULT_ANALYTICS_STATE,

  setView: (view) => set({ activeView: view }),

  setFilter: (key, value) => set({ [key]: value }),

  resetFilters: () =>
    set({
      nodeTypeFilter: DEFAULT_ANALYTICS_STATE.nodeTypeFilter,
      couplingThreshold: DEFAULT_ANALYTICS_STATE.couplingThreshold,
      showCycles: DEFAULT_ANALYTICS_STATE.showCycles,
      showHotspots: DEFAULT_ANALYTICS_STATE.showHotspots,
    }),

  resetAnalytics: () => set({ ...DEFAULT_ANALYTICS_STATE }),
}))

// Selectors
export const useActiveView = () => useAnalyticsStore((s) => s.activeView)
export const useNodeTypeFilter = () => useAnalyticsStore((s) => s.nodeTypeFilter)
export const useCouplingThreshold = () => useAnalyticsStore((s) => s.couplingThreshold)
export const useShowCycles = () => useAnalyticsStore((s) => s.showCycles)
export const useShowHotspots = () => useAnalyticsStore((s) => s.showHotspots)
