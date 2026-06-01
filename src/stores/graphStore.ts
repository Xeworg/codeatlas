import { create } from 'zustand'
import type { GraphData, GraphNode } from '../lib/types'

interface GraphState {
  // State
  graphData: GraphData | null
  selectedNodeId: string | null
  hoveredNodeId: string | null
  searchQuery: string
  searchResults: GraphNode[]
  isLoading: boolean
  error: string | null

  // Actions
  setGraphData: (data: GraphData) => void
  selectNode: (nodeId: string | null) => void
  setHoveredNode: (nodeId: string | null) => void
  setSearchQuery: (query: string) => void
  setSearchResults: (results: GraphNode[]) => void
  setLoading: (loading: boolean) => void
  setError: (error: string | null) => void
  clearGraph: () => void
}

export const useGraphStore = create<GraphState>((set) => ({
  graphData: null,
  selectedNodeId: null,
  hoveredNodeId: null,
  searchQuery: '',
  searchResults: [],
  isLoading: false,
  error: null,

  setGraphData: (data) => set({ graphData: data, error: null }),

  selectNode: (nodeId) => set({ selectedNodeId: nodeId }),

  setHoveredNode: (nodeId) => set({ hoveredNodeId: nodeId }),

  setSearchQuery: (query) => set({ searchQuery: query }),

  setSearchResults: (results) => set({ searchResults: results }),

  setLoading: (loading) => set({ isLoading: loading }),

  setError: (error) => set({ error, isLoading: false }),

  clearGraph: () =>
    set({
      graphData: null,
      selectedNodeId: null,
      hoveredNodeId: null,
      searchQuery: '',
      searchResults: [],
      isLoading: false,
      error: null,
    }),
}))

// Selectors
export const useGraphData = () => useGraphStore((s) => s.graphData)
export const useSelectedNodeId = () => useGraphStore((s) => s.selectedNodeId)
export const useIsGraphLoading = () => useGraphStore((s) => s.isLoading)
