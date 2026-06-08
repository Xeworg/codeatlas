// useAIConfig — hook for AI provider configuration
// Part of PR-8 (Frontend services/hooks)

import { useState, useCallback } from 'react'
import { configureAI as _configureAI, getAIConfig as _getAIConfig } from '../services/aiService'
import type { AIConfig } from '../lib/types'

interface UseAIConfigResult {
  config: Omit<AIConfig, 'api_key'> | null
  loading: boolean
  saving: boolean
  error: string | null
  save: (config: AIConfig) => Promise<void>
  load: () => Promise<void>
}

/**
 * Manage AI provider configuration (save and load).
 */
export function useAIConfig(): UseAIConfigResult {
  const [config, setConfig] = useState<Omit<AIConfig, 'api_key'> | null>(null)
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const result = await _getAIConfig()
      setConfig(result)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load AI config')
    } finally {
      setLoading(false)
    }
  }, [])

  const save = useCallback(async (cfg: AIConfig) => {
    setSaving(true)
    setError(null)
    try {
      await _configureAI(cfg)
      setConfig({ provider: cfg.provider, model: cfg.model, endpoint: cfg.endpoint })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save AI config')
      throw e
    } finally {
      setSaving(false)
    }
  }, [])

  return { config, loading, saving, error, save, load }
}
