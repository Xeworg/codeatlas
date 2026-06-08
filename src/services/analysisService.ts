// analysisService — domain-oriented wrapper for analysis/export operations
// Part of PR-8 (Frontend services/hooks)
// Bridges the gap between components and tauri-api for analysis features.

import { exportView as _exportView, getGraphInsights as _getGraphInsights } from '../lib/tauri-api'
import type { ExportPayload, GraphInsights } from '../lib/types'

// ─── Export ─────────────────────────────────────────────────────────────────

/**
 * Export the current graph view as JSON.
 * PNG generation is handled client-side; this returns JSON payload.
 */
export async function exportView(
  projectId: string,
  format: 'json' | 'png'
): Promise<ExportPayload> {
  return _exportView(projectId, format)
}

// ─── Graph insights ──────────────────────────────────────────────────────────

/**
 * Retrieve or compute graph insights (cycles, hotspots, coupling metrics)
 * for the project. Results may be cached on the backend.
 */
export async function getGraphInsights(projectId: string): Promise<GraphInsights> {
  return _getGraphInsights(projectId)
}
