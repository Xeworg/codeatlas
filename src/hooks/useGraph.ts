// useGraph — hook for graph data, selection and search
// Part of PR-8 (Frontend services/hooks)
import { useCallback } from 'react'
import { useGraphStore } from '../stores/graphStore'
import { getGraph, searchNodes } from '@/lib/tauri-api'
import { buildLayout } from '../lib/graph-layout'
import { useProjectId } from '../stores/projectStore'

export function useGraph() {
  const projectId = useProjectId()
  const {
    graphData,
    selectedNodeId,
    hoveredNodeId,
    searchQuery,
    searchResults,
    isLoading,
    error,
    setGraphData,
    selectNode,
    setHoveredNode,
    setSearchQuery,
    setSearchResults,
    setLoading,
    setError,
    clearGraph,
  } = useGraphStore()

  const loadGraph = useCallback(async () => {
    if (!projectId) return
    setLoading(true)
    setError(null)
    try {
      const raw = await getGraph(projectId)
      const laid = buildLayout(raw)
      setGraphData(laid)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }, [projectId, setLoading, setError, setGraphData])

  const search = useCallback(
    async (query: string) => {
      setSearchQuery(query)
      if (!query.trim() || !projectId) {
        setSearchResults([])
        return
      }
      try {
        const results = await searchNodes(projectId, query)
        setSearchResults(results)
      } catch {
        setSearchResults([])
      }
    },
    [projectId, setSearchQuery, setSearchResults]
  )

  const clearSelection = useCallback(() => {
    selectNode(null)
  }, [selectNode])

  return {
    graphData,
    selectedNodeId,
    hoveredNodeId,
    searchQuery,
    searchResults,
    isLoading,
    error,
    loadGraph,
    selectNode,
    setHoveredNode,
    search,
    clearSelection,
    clearGraph,
  }
}
