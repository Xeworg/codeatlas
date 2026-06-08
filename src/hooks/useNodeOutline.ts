// useNodeOutline — hook for loading code outline for a node
// Part of PR-8 (Frontend services/hooks)

import { useState, useEffect, useCallback } from 'react'
import { getNodeOutline as _getNodeOutline } from '../services/graphService'
import type { OutlineItem } from '../lib/types'

interface UseNodeOutlineResult {
  outline: OutlineItem[]
  loading: boolean
  error: string | null
  reload: () => void
}

/**
 * Load the code outline (symbols, functions, classes) for a given node ID.
 * Automatically reloads when nodeId changes.
 */
export function useNodeOutline(nodeId: string | null): UseNodeOutlineResult {
  const [outline, setOutline] = useState<OutlineItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [version, setVersion] = useState(0)

  const reload = useCallback(() => {
    setVersion((v) => v + 1)
  }, [])

  useEffect(() => {
    if (!nodeId) {
      setOutline([])
      setError(null)
      return
    }

    let cancelled = false
    setLoading(true)
    setError(null)

    _getNodeOutline(nodeId)
      .then((items) => {
        if (!cancelled) setOutline(items)
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message || 'Failed to load outline')
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [nodeId, version])

  return { outline, loading, error, reload }
}
