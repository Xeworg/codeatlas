// useExport hook — manages export state (JSON/PNG)
// Part of PR-8 (Frontend services/hooks)

import { useState, useCallback, useRef } from 'react'
import { exportView } from '@/lib/tauri-api'
import { t } from '../lib/i18n'
import type { ExportPayload } from '../lib/types'

type ExportStatus = 'idle' | 'exporting' | 'done' | 'error'

interface UseExportReturn {
  status: ExportStatus
  error: string | null
  fallbackWarning: string | null
  exportJson: (projectId: string) => Promise<void>
  exportPng: (graphElement: HTMLElement | null, projectId: string) => Promise<void>
  reset: () => void
}

export function useExport(): UseExportReturn {
  const [status, setStatus] = useState<ExportStatus>('idle')
  const [error, setError] = useState<string | null>(null)
  const [fallbackWarning, setFallbackWarning] = useState<string | null>(null)
  const objectUrlRef = useRef<string | null>(null)

  const cleanup = useCallback(() => {
    if (objectUrlRef.current) {
      URL.revokeObjectURL(objectUrlRef.current)
      objectUrlRef.current = null
    }
  }, [])

  const triggerDownload = useCallback(
    (blob: Blob, filename: string) => {
      cleanup()
      const url = URL.createObjectURL(blob)
      objectUrlRef.current = url
      const a = document.createElement('a')
      a.href = url
      a.download = filename
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
    },
    [cleanup]
  )

  const doJsonExport = useCallback(
    async (projectId: string) => {
      const payload: ExportPayload = await exportView(projectId, 'json')
      const jsonStr = JSON.stringify(payload, null, 2)
      const blob = new Blob([jsonStr], { type: 'application/json' })
      triggerDownload(blob, `codeatlas-export-${projectId}.json`)
    },
    [triggerDownload]
  )

  const exportJson = useCallback(
    async (projectId: string) => {
      setStatus('exporting')
      setError(null)
      setFallbackWarning(null)
      cleanup()

      try {
        await doJsonExport(projectId)
        setStatus('done')
      } catch (err) {
        const msg = err instanceof Error ? err.message : t('export.jsonError')
        setError(msg)
        setStatus('error')
      }
    },
    [cleanup, doJsonExport]
  )

  const exportPng = useCallback(
    async (graphElement: HTMLElement | null, projectId: string) => {
      setStatus('exporting')
      setError(null)
      setFallbackWarning(null)
      cleanup()

      if (!graphElement) {
        // No graph element → fallback to JSON
        setFallbackWarning(t('export.noGraphFallback'))
        try {
          await doJsonExport(projectId)
          setStatus('done')
        } catch (jsonErr) {
          const msg = jsonErr instanceof Error ? jsonErr.message : t('export.jsonError')
          setError(msg)
          setStatus('error')
        }
        return
      }

      try {
        // Dynamically import html-to-image to avoid SSR issues
        const { toBlob } = await import('html-to-image')
        const blob = await toBlob(graphElement, { backgroundColor: '#1e1e1e' })

        if (!blob) {
          throw new Error('html-to-image devolvió null')
        }

        triggerDownload(blob, `codeatlas-graph-${projectId}.png`)
        setStatus('done')
      } catch (err) {
        // Fallback to JSON on PNG failure
        const pngError = err instanceof Error ? err.message : 'Error desconocido'
        setFallbackWarning(t('export.pngFallback', { error: pngError }))
        try {
          await doJsonExport(projectId)
          setStatus('done')
        } catch (jsonErr) {
          const msg = jsonErr instanceof Error ? jsonErr.message : t('export.jsonError')
          setError(msg)
          setStatus('error')
        }
      }
    },
    [cleanup, doJsonExport, triggerDownload]
  )

  const reset = useCallback(() => {
    cleanup()
    setStatus('idle')
    setError(null)
    setFallbackWarning(null)
  }, [cleanup])

  return { status, error, fallbackWarning, exportJson, exportPng, reset }
}
