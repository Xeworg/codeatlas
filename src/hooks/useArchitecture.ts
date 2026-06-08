// useArchitecture — hook for architecture detection and impact analysis
// Part of PR-8 (Frontend services/hooks)
// Manages async fetching of analysis data with proper loading/error states.

import { useState, useEffect, useRef, useCallback } from 'react'
import {
  getArchitectureDetection as _getArchitectureDetection,
  getImpactAnalysis as _getImpactAnalysis,
} from '../services/projectService'
import type { ArchitectureDetectionResult, ImpactAnalysisResult } from '../lib/types'
import { useProjectStore } from '../stores/projectStore'
import { useGraphStore } from '../stores/graphStore'
import { V3_H1_ENABLED } from '../stores/featureFlags'

interface UseArchitectureReturn {
  architectureDetection: ArchitectureDetectionResult | null
  impactAnalysis: ImpactAnalysisResult | null
  loadingDetection: boolean
  loadingImpact: boolean
}

export function useArchitecture(): UseArchitectureReturn {
  const status = useProjectStore((s) => s.status)
  const projectId = useProjectStore((s) => s.projectId)
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId)

  const [architectureDetection, setArchitectureDetection] =
    useState<ArchitectureDetectionResult | null>(null)
  const [impactAnalysis, setImpactAnalysis] = useState<ImpactAnalysisResult | null>(null)

  const prevProjectId = useRef<string | null>(null)

  // ── Fetch architecture detection when project becomes ready ─────────────
  const fetchArchitecture = useCallback(async (pid: string) => {
    setArchitectureDetection(null)
    try {
      const result = await _getArchitectureDetection(pid)
      setArchitectureDetection(result)
    } catch {
      setArchitectureDetection(null)
    }
  }, [])

  // ── Fetch impact analysis when node is selected ────────────────────────
  const fetchImpact = useCallback(async (pid: string, nid: string) => {
    setImpactAnalysis(null)
    try {
      const result = await _getImpactAnalysis(pid, nid)
      setImpactAnalysis(result)
    } catch {
      setImpactAnalysis(null)
    }
  }, [])

  useEffect(() => {
    if (!V3_H1_ENABLED) return
    if (status !== 'ready' || !projectId) return
    if (projectId === prevProjectId.current) return
    prevProjectId.current = projectId
    fetchArchitecture(projectId)
  }, [status, projectId, fetchArchitecture])

  useEffect(() => {
    if (!V3_H1_ENABLED) return
    if (status !== 'ready' || !projectId || !selectedNodeId) {
      setImpactAnalysis(null)
      return
    }
    fetchImpact(projectId, selectedNodeId)
  }, [status, projectId, selectedNodeId, fetchImpact])

  return {
    architectureDetection,
    impactAnalysis,
    loadingDetection: V3_H1_ENABLED && status === 'ready' && !architectureDetection,
    loadingImpact: V3_H1_ENABLED && !!selectedNodeId && !impactAnalysis,
  }
}
