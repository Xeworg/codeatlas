// useNodeDetails — hook for loading node/file details
// Part of PR-8 (Frontend services/hooks)

import { useState, useEffect, useCallback } from 'react'
import { getNodeDetails } from '@/lib/tauri-api'
import type { FileInfo } from '../lib/types'

interface UseNodeDetailsResult {
  details: FileInfo | null
  loading: boolean
  error: string | null
  reload: () => void
}

/**
 * Load file details for a given node ID.
 * Automatically reloads when nodeId changes.
 */
export function useNodeDetails(nodeId: string | null): UseNodeDetailsResult {
  const [details, setDetails] = useState<FileInfo | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [version, setVersion] = useState(0)

  const reload = useCallback(() => {
    setVersion((v) => v + 1)
  }, [])

  useEffect(() => {
    if (!nodeId) {
      setDetails(null)
      setError(null)
      return
    }

    let cancelled = false
    setLoading(true)
    setError(null)

    getNodeDetails(nodeId)
      .then((data) => {
        if (!cancelled) setDetails(data)
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message || 'Failed to load node details')
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })

    return () => {
      cancelled = true
    }
  }, [nodeId, version])

  return { details, loading, error, reload }
}
