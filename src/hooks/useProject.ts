// useProject — hook for project open/reopen/scan orchestration
// Part of PR-8 (Frontend services/hooks)
// Encapsulates the open-or-scan workflow and state management.

import { useCallback } from 'react'
import { useProjectStore } from '../stores/projectStore'
import { useGraphStore } from '../stores/graphStore'
import {
  scanProject as _scanProject,
  openProjectByPath as _openProjectByPath,
} from '../services/projectService'
import { getGraph as _getGraph } from '../services/graphService'
import { getErrorMessage } from '../lib/tauri-api'
import { buildLayout } from '../lib/graph-layout'
import type { ScanResult } from '../lib/types'

interface UseProjectReturn {
  openProject: (path: string) => Promise<void>
  loading: boolean
}

/**
 * Encapsulates project open/reopen/scan orchestration.
 * Replaces inline orchestration in App.tsx.
 *
 * Flow:
 * 1. Try to reopen an already-indexed project
 * 2. If not found, initiate a fresh scan
 * 3. On success, load the graph
 */
export function useProject(): UseProjectReturn {
  const { setProject, setScanResult, setStatus, setError } = useProjectStore()
  const { setGraphData, setLoading } = useGraphStore()

  const openProject = useCallback(
    async (path: string) => {
      const name = path.split('/').pop() ?? 'Project'
      setLoading(true)

      // Step 1: Try to reopen an already-indexed project
      let reopenResult: ScanResult | null = null
      try {
        reopenResult = await _openProjectByPath(path)
      } catch (reopenErr) {
        const reopenErrMsg = getErrorMessage(reopenErr)
        if (!reopenErrMsg.includes('No project found')) {
          setError(reopenErrMsg)
          setLoading(false)
          return
        }
      }

      if (reopenResult) {
        // Project was already indexed — load it
        setScanResult(reopenResult)
        setStatus(reopenResult.status)
        if (reopenResult.projectId) {
          setProject(
            reopenResult.projectId,
            reopenResult.projectName || name,
            reopenResult.rootPath || path
          )
        }
        if (reopenResult.status === 'ready' && reopenResult.projectId) {
          try {
            const graph = await _getGraph(reopenResult.projectId)
            const laid = buildLayout(graph)
            setGraphData(laid)
          } catch (graphErr) {
            setError(`Graph load failed: ${getErrorMessage(graphErr)}`)
            setLoading(false)
            return
          }
        }
        setLoading(false)
        return
      }

      // Step 2: Fresh scan for paths not yet indexed
      setProject(`proj-${Date.now()}`, name, path)
      setStatus('scanning')

      try {
        const result = await _scanProject(path)
        setScanResult(result)
        setStatus(result.status)

        if (result.projectId) {
          setProject(result.projectId, result.projectName || name, result.rootPath || path)
        }

        if (result.status === 'ready' && result.projectId) {
          setLoading(true)
          try {
            const graph = await _getGraph(result.projectId)
            const laid = buildLayout(graph)
            setGraphData(laid)
          } catch (e) {
            setError(`Graph load failed: ${getErrorMessage(e)}`)
          } finally {
            setLoading(false)
          }
        } else {
          setLoading(false)
        }
      } catch (err) {
        setError(getErrorMessage(err))
        setLoading(false)
      }
    },
    [setProject, setStatus, setScanResult, setError, setGraphData, setLoading]
  )

  const loading = useGraphStore((s) => s.isLoading)

  return { openProject, loading }
}
