// graphService — domain-oriented wrapper for graph operations
// Part of PR-8 (Frontend services/hooks)
// Bridges the gap between components and tauri-api for graph data access.

import {
  getGraph as _getGraph,
  getNodeDetails as _getNodeDetails,
  getNodeOutline as _getNodeOutline,
  searchNodes as _searchNodes,
  getDependencies as _getDependencies,
  getDependents as _getDependents,
} from '../lib/tauri-api'
import type { GraphData, FileInfo, OutlineItem, GraphNode } from '../lib/types'

// ─── Graph data ─────────────────────────────────────────────────────────────

/**
 * Load the full graph for a project.
 * May return a cached result from the backend.
 */
export async function getGraph(projectId: string): Promise<GraphData> {
  return _getGraph(projectId)
}

// ─── Node metadata ──────────────────────────────────────────────────────────

/**
 * Load detailed file information for a specific node.
 */
export async function getNodeDetails(nodeId: string): Promise<FileInfo> {
  return _getNodeDetails(nodeId)
}

/**
 * Load the code outline (symbols, functions, classes) for a specific node.
 */
export async function getNodeOutline(nodeId: string): Promise<OutlineItem[]> {
  return _getNodeOutline(nodeId)
}

// ─── Search ────────────────────────────────────────────────────────────────

/**
 * Search nodes by name/path across the project graph.
 */
export async function searchNodes(
  projectId: string,
  query: string,
  limit = 20
): Promise<GraphNode[]> {
  return _searchNodes(projectId, query, limit)
}

// ─── Dependency graph ───────────────────────────────────────────────────────

/**
 * Get outgoing dependencies from a node.
 */
export async function getDependencies(
  nodeId: string
): Promise<Array<{ id: string; source: string; target: string; imports: string[] }>> {
  return _getDependencies(nodeId)
}

/**
 * Get incoming dependencies (dependents) for a node.
 */
export async function getDependents(
  nodeId: string
): Promise<Array<{ id: string; source: string; target: string; imports: string[] }>> {
  return _getDependents(nodeId)
}
