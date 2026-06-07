// projectService — domain-oriented wrapper for project/scan operations
// Part of PR-8 (Frontend services/hooks)
// Bridges the gap between components and tauri-api for project management.

import {
  scanProject as _scanProject,
  openProjectByPath as _openProjectByPath,
  getScanStatus as _getScanStatus,
  cancelScan as _cancelScan,
  getArchitectureDetection as _getArchitectureDetection,
  getImpactAnalysis as _getImpactAnalysis,
} from '../lib/tauri-api'
import type { ScanResult, ScanStatus } from '../lib/types'
import type { ArchitectureDetectionResult, ImpactAnalysisResult } from '../lib/types'

// ─── Project & Scanning ──────────────────────────────────────────────────────

/**
 * Initiate a full project scan at the given filesystem path.
 * Returns a ScanResult with project metadata and initial file list.
 */
export async function scanProject(path: string): Promise<ScanResult> {
  return _scanProject(path)
}

/**
 * Re-open an already-indexed project without re-scanning.
 * Returns null if the path has never been scanned.
 */
export async function openProjectByPath(path: string): Promise<ScanResult> {
  return _openProjectByPath(path)
}

/**
 * Read the current in-memory scan status and progress.
 */
export async function getScanStatus(): Promise<{ status: ScanStatus; progress: number }> {
  return _getScanStatus()
}

/**
 * Cancel an in-progress scan for the given project.
 */
export async function cancelScan(projectId: string): Promise<void> {
  return _cancelScan(projectId)
}

// ─── Architecture Analysis ──────────────────────────────────────────────────

/**
 * Request architecture detection for a project.
 * Results may be cached on the backend.
 */
export async function getArchitectureDetection(
  projectId: string
): Promise<ArchitectureDetectionResult> {
  return _getArchitectureDetection(projectId)
}

/**
 * Compute impact analysis for a specific node in the project graph.
 */
export async function getImpactAnalysis(
  projectId: string,
  nodeId: string
): Promise<ImpactAnalysisResult> {
  return _getImpactAnalysis(projectId, nodeId)
}
