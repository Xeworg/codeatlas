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
  OutlineItem,
} from './types'
import type { ApiError, ErrorCode } from './types'

// MARK: Error helpers

/**
 * Backend-to-frontend error code mapping.
 *
 * The backend emits structured JSON with these codes:
 * - PROJECT_NOT_FOUND -> PATH_NOT_FOUND
 * - FILE_NOT_FOUND -> PATH_NOT_FOUND
 * - AI_UNAVAILABLE -> UNREACHABLE
 * - AI_RATE_LIMITED -> RATE_LIMITED
 * - AI_TOKEN_LIMIT -> TOKEN_LIMIT
 * - INVALID_API_KEY -> INVALID_KEY
 * - ACCESS_DENIED -> ACCESS_DENIED
 * - SCAN_TIMEOUT -> SCAN_TIMEOUT
 * - DATABASE -> INTERNAL
 * - INTERNAL -> INTERNAL
 */
const BACKEND_TO_FRONTEND_CODE: Record<string, ErrorCode> = {
  PROJECT_NOT_FOUND: 'PATH_NOT_FOUND',
  FILE_NOT_FOUND: 'PATH_NOT_FOUND',
  AI_UNAVAILABLE: 'UNREACHABLE',
  AI_RATE_LIMITED: 'RATE_LIMITED',
  AI_TOKEN_LIMIT: 'TOKEN_LIMIT',
  INVALID_API_KEY: 'INVALID_KEY',
  ACCESS_DENIED: 'ACCESS_DENIED',
  SCAN_TIMEOUT: 'SCAN_TIMEOUT',
  DATABASE: 'INTERNAL',
  INTERNAL: 'INTERNAL',
}

/**
 * Convert any thrown error to a typed ApiError.
 *
 * This function implements the structured error contract:
 * 1. Attempts to parse the error message as JSON with { code, message, details }
 * 2. If successful and has valid code/message, maps backend code to frontend and returns
 * 3. Falls back to legacy string heuristics for backward compatibility
 *
 * The details field is preserved as structured data (Record<string, unknown> | undefined).
 */
export function toApiError(err: unknown, fallbackCode: ErrorCode = 'INTERNAL'): ApiError {
  const raw = err instanceof Error ? err.message : String(err)

  // Strip Tauri "Error: " prefix before JSON parsing (Tauri wraps all errors)
  const msg = raw.startsWith('Error: ') ? raw.slice(7) : raw

  // Try to parse structured JSON from the error message first
  try {
    const parsed = JSON.parse(msg)
    if (
      typeof parsed === 'object' &&
      parsed !== null &&
      'code' in parsed &&
      'message' in parsed &&
      typeof parsed.code === 'string' &&
      typeof parsed.message === 'string'
    ) {
      const backendCode = parsed.code
      const frontendCode = BACKEND_TO_FRONTEND_CODE[backendCode] ?? fallbackCode

      // Preserve details as structured data, or undefined if null/missing
      const details =
        parsed.details && typeof parsed.details === 'object' && parsed.details !== null
          ? (parsed.details as Record<string, unknown>)
          : undefined

      return {
        code: frontendCode,
        message: parsed.message,
        ...(details !== undefined && { details }),
      }
    }
  } catch {
    // JSON parsing failed - fall through to legacy heuristics
  }

  // Legacy fallback: string-based pattern matching for backward compatibility
  let code: ErrorCode = fallbackCode
  if (msg.includes('ENOENT') || msg.includes('not found') || msg.includes('PATH_NOT_FOUND')) {
    code = 'PATH_NOT_FOUND'
  } else if (msg.includes('already exists') || msg.includes('Project already exists')) {
    code = 'PROJECT_EXISTS'
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

/**
 * Translate an error code to a user-friendly Spanish message.
 *
 * Used by hooks and components to display localized, human-readable
 * error messages instead of raw backend messages or error codes.
 */
export function toUserMessage(apiError: { code: ErrorCode; message?: string }): string {
  switch (apiError.code) {
    case 'INVALID_KEY':
      return 'La clave de API no es válida. Verificá que esté correcta y no haya expirado.'
    case 'RATE_LIMITED':
      return 'Se excedió el límite de solicitudes. Esperá un momento antes de intentar de nuevo.'
    case 'TOKEN_LIMIT':
      return 'El contexto es demasiado largo para el modelo. Intentá con un nodo diferente.'
    case 'UNREACHABLE':
      return 'No se pudo conectar al proveedor de IA. Verificá tu conexión a internet.'
    case 'PATH_NOT_FOUND':
      return 'No se encontró el archivo o proyecto solicitado.'
    case 'PROJECT_EXISTS':
      return 'Ya existe un proyecto en esta ubicación.'
    case 'ACCESS_DENIED':
      return 'No tenés permisos para acceder a este recurso.'
    case 'SCAN_TIMEOUT':
      return 'El escaneo tardó demasiado y fue cancelado. Intentá con un proyecto más pequeño.'
    case 'INTERNAL':
    default:
      return apiError.message || 'Ocurrió un error inesperado. Intentá de nuevo.'
  }
}

/**
 * Extract a human-readable message from any thrown value.
 * Handles Error instances, plain objects, strings, and null/undefined.
 * Preserves `{ code, message }` shape for downstream consumers.
 */
export function getErrorMessage(err: unknown): string {
  if (err === null || err === undefined) return 'Unknown error'
  if (typeof err === 'string') return err
  if (typeof err === 'object' && 'message' in err) {
    const msg = String((err as Record<string, unknown>).message)
    if ('code' in err && typeof (err as Record<string, unknown>).code === 'string') {
      return `${(err as Record<string, unknown>).code} — ${msg}`
    }
    return msg
  }
  return 'Unknown error'
}

// MARK: Project & Scanning ---

export async function scanProject(path: string): Promise<ScanResult> {
  try {
    return await invoke<ScanResult>('scan_project', { path })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

// MARK: Project reopen / load by path ---

export async function openProjectByPath(path: string): Promise<ScanResult> {
  try {
    return await invoke<ScanResult>('open_project_by_path', { path })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
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

export async function getNodeOutline(nodeId: string): Promise<OutlineItem[]> {
  try {
    return await invoke<OutlineItem[]>('get_node_outline', { nodeId })
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
  ExecutiveArchitectureSummary,
  SnapshotDiffPayload,
  C4ViewPayload,
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

export async function listWorkspaceProjects(workspaceId: string): Promise<WorkspaceProject[]> {
  try {
    return await invoke<WorkspaceProject[]>('list_workspace_projects', { workspaceId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

// ── Snapshots (PR5: full payload capture) ──────────────────────────────

export async function createSnapshot(
  projectId: string,
  label: string,
  workspaceId?: string
): Promise<Snapshot> {
  try {
    return await invoke<Snapshot>('create_snapshot', { projectId, label, workspaceId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function getSnapshot(snapshotId: string): Promise<Snapshot | null> {
  try {
    return await invoke<Snapshot | null>('get_snapshot', { snapshotId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function listSnapshots(projectId: string, workspaceId?: string): Promise<Snapshot[]> {
  try {
    return await invoke<Snapshot[]>('list_snapshots', { projectId, workspaceId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

// ── Annotations (PR6) ─────────────────────────────────────────────────────

export interface Annotation {
  id: string
  projectId: string
  nodeId: string
  author: string
  kind: 'comment' | 'todo' | 'review' | 'issue'
  text: string
  createdAt: string
}

export async function addComment(
  projectId: string,
  nodeId: string,
  author: string,
  text: string,
  kind?: string
): Promise<Annotation> {
  try {
    return await invoke<Annotation>('add_comment', { projectId, nodeId, author, text, kind })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function listComments(projectId: string, nodeId?: string): Promise<Annotation[]> {
  try {
    return await invoke<Annotation[]>('list_comments', { projectId, nodeId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export interface HealthTimeline {
  records: HealthRecord[]
  projectId: string
  from: string
  to: string
}

export interface HealthRecord {
  id: string
  recordedAt: string
  overallScore: number
  couplingScore: number
  complexityScore: number
  cycleCount: number
  hotspotCount: number
}

export async function getHealthTimeline(
  projectId: string,
  from: string,
  to: string
): Promise<HealthTimeline> {
  try {
    return await invoke<HealthTimeline>('get_health_timeline', { projectId, from, to })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

// ========================================================================
// H3 — Executive Summary + Diff + C4 Views
// ========================================================================

export async function getExecutiveSummary(
  workspaceId: string
): Promise<ExecutiveArchitectureSummary> {
  try {
    return await invoke<ExecutiveArchitectureSummary>('get_executive_summary', { workspaceId })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function compareSnapshots(
  baseSnapshotId: string,
  targetSnapshotId: string
): Promise<SnapshotDiffPayload> {
  try {
    return await invoke<SnapshotDiffPayload>('compare_snapshots', {
      baseSnapshotId,
      targetSnapshotId,
    })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}

export async function getC4View(projectId: string, level: 1 | 2): Promise<C4ViewPayload> {
  try {
    return await invoke<C4ViewPayload>('get_c4_view', { projectId, level })
  } catch (e) {
    throw toApiError(e, 'INTERNAL')
  }
}
