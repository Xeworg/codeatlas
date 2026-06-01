// Tauri API wrappers — typed invoke calls
// Canonical contract between frontend and Rust backend.

import { invoke } from '@tauri-apps/api/core'
import type {
  ScanResult,
  GraphData,
  FileInfo,
  GraphNode,
  NodeExplanation,
  ChatResponse,
  AIConfig,
  ScanStatus,
} from './types'
import type { ApiError, ErrorCode } from './types'

// MARK: Error helpers

function toApiError(err: unknown, fallbackCode: ErrorCode = 'INTERNAL'): ApiError {
  const msg = err instanceof Error ? err.message : String(err)
  let code: ErrorCode = fallbackCode
  if (msg.includes('ENOENT') || msg.includes('not found') || msg.includes('PATH_NOT_FOUND')) {
    code = 'PATH_NOT_FOUND'
  } else if (msg.includes('EACCES') || msg.includes('ACCESS_DENIED')) {
    code = 'ACCESS_DENIED'
  } else if (msg.includes('timeout') || msg.includes('TIMEOUT')) {
    code = 'SCAN_TIMEOUT'
  } else if (
    msg.includes('401') ||
    msg.includes('InvalidApiKey') ||
    msg.includes('invalid_api_key')
  ) {
    code = 'INVALID_KEY'
  } else if (msg.includes('429') || msg.includes('rate_limit') || msg.includes('RATE_LIMITED')) {
    code = 'RATE_LIMITED'
  } else if (
    msg.includes('TOKEN_LIMIT') ||
    msg.includes('token_limit') ||
    msg.includes('context_length')
  ) {
    code = 'TOKEN_LIMIT'
  } else if (
    msg.includes('ECONNREFUSED') ||
    msg.includes('UNREACHABLE') ||
    msg.includes('network')
  ) {
    code = 'UNREACHABLE'
  }
  return { code, message: msg }
}

// MARK: Project & Scanning ---

export async function scanProject(path: string): Promise<ScanResult> {
  try {
    return await invoke<ScanResult>('scan_project', { path })
  } catch (e) {
    throw toApiError(e, 'PATH_NOT_FOUND')
  }
}

export async function getScanStatus(): Promise<{ status: ScanStatus; progress: number }> {
  try {
    return await invoke<{ status: ScanStatus; progress: number }>('get_scan_status')
  } catch (e) {
    throw toApiError(e)
  }
}

export async function cancelScan(projectId: string): Promise<void> {
  try {
    await invoke<void>('cancel_scan', { projectId })
  } catch (e) {
    throw toApiError(e)
  }
}

// MARK: Graph ---

export async function getGraph(projectId: string): Promise<GraphData> {
  try {
    return await invoke<GraphData>('get_graph', { projectId })
  } catch (e) {
    throw toApiError(e)
  }
}

export async function getNodeDetails(nodeId: string): Promise<FileInfo> {
  try {
    return await invoke<FileInfo>('get_node_details', { nodeId })
  } catch (e) {
    throw toApiError(e)
  }
}

export async function searchNodes(
  projectId: string,
  query: string,
  limit = 20
): Promise<GraphNode[]> {
  try {
    return await invoke<GraphNode[]>('search_nodes', { projectId, query, limit })
  } catch (e) {
    throw toApiError(e)
  }
}

export async function getDependencies(
  nodeId: string
): Promise<{ id: string; source: string; target: string; imports: string[] }[]> {
  try {
    return await invoke<{ id: string; source: string; target: string; imports: string[] }[]>(
      'get_dependencies',
      { nodeId }
    )
  } catch (e) {
    throw toApiError(e)
  }
}

export async function getDependents(
  nodeId: string
): Promise<{ id: string; source: string; target: string; imports: string[] }[]> {
  try {
    return await invoke<{ id: string; source: string; target: string; imports: string[] }[]>(
      'get_dependents',
      { nodeId }
    )
  } catch (e) {
    throw toApiError(e)
  }
}

// MARK: AI ---

export async function configureAI(config: AIConfig): Promise<void> {
  try {
    await invoke<void>('configure_ai', { config })
  } catch (e) {
    throw toApiError(e, 'INVALID_KEY')
  }
}

export async function getAIConfig(): Promise<Omit<AIConfig, 'api_key'>> {
  try {
    return await invoke<Omit<AIConfig, 'api_key'>>('get_ai_config')
  } catch (e) {
    throw toApiError(e)
  }
}

export async function explainNode(nodeId: string, projectId: string): Promise<NodeExplanation> {
  try {
    return await invoke<NodeExplanation>('explain_node', { nodeId, projectId })
  } catch (e) {
    throw toApiError(e, 'INVALID_KEY')
  }
}

export async function chat(
  projectId: string,
  message: string,
  history: { id: string; role: string; content: string; timestamp: string }[],
  contextNodeIds?: string[]
): Promise<ChatResponse> {
  try {
    return await invoke<ChatResponse>('chat', {
      projectId,
      message,
      history,
      contextNodeIds,
    })
  } catch (e) {
    throw toApiError(e, 'INVALID_KEY')
  }
}

// MARK: v2 Analysis Commands

import type {
  ArchitectureDetectionResult,
  ImpactAnalysisResult,
  GraphInsights,
  ExportPayload,
} from './types'

// MARK: v3 Workspace Commands

import type {
  Workspace,
  WorkspaceProject,
  Snapshot,
} from './types-v3'

// MARK: v2 Analysis Commands

/**
 * Request architecture detection for a project.
 * Returns the detected pattern, confidence score, and supporting evidence.
 */
export async function getArchitectureDetection(
  projectId: string
): Promise<ArchitectureDetectionResult> {
  try {
    return await invoke<ArchitectureDetectionResult>('get_architecture_detection', { projectId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

/**
 * Compute impact analysis for a specific node in the project graph.
 * Returns the list of affected nodes and an impact score.
 */
export async function getImpactAnalysis(
  projectId: string,
  nodeId: string
): Promise<ImpactAnalysisResult> {
  try {
    return await invoke<ImpactAnalysisResult>('get_impact_analysis', { projectId, nodeId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

/**
 * Retrieve or compute graph insights (cycles, hotspots, coupling metrics)
 * for the project. Results may be cached on the backend.
 */
export async function getGraphInsights(projectId: string): Promise<GraphInsights> {
  try {
    return await invoke<GraphInsights>('get_graph_insights', { projectId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

/**
 * Export the current graph view as JSON or PNG.
 * PNG generation is handled by the frontend with a fallback to JSON on failure.
 */
export async function exportView(
  projectId: string,
  format: 'json' | 'png'
): Promise<ExportPayload> {
  try {
    if (format === 'json') {
      return await invoke<ExportPayload>('export_view', { projectId, format })
    }
    throw Object.assign(new Error('PNG format handled client-side'), { code: 'INTERNAL' })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

// MARK: v3 Workspace Commands

export async function createWorkspace(name: string): Promise<Workspace> {
  try {
    return await invoke<Workspace>('create_workspace', { name })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function listWorkspaces(): Promise<Workspace[]> {
  try {
    return await invoke<Workspace[]>('list_workspaces')
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function attachProjectToWorkspace(
  workspaceId: string,
  projectId: string
): Promise<void> {
  try {
    await invoke<void>('attach_project_to_workspace', { workspaceId, projectId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function listWorkspaceProjects(
  workspaceId: string
): Promise<WorkspaceProject[]> {
  try {
    return await invoke<WorkspaceProject[]>('list_workspace_projects', { workspaceId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

// ── Snapshot stubs (full payload capture in PR5) ─────────────────────────

export async function createSnapshot(
  projectId: string,
  label: string
): Promise<Snapshot> {
  try {
    return await invoke<Snapshot>('create_snapshot', { projectId, label })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function listSnapshots(projectId: string): Promise<Snapshot[]> {
  try {
    return await invoke<Snapshot[]>('list_snapshots', { projectId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}
