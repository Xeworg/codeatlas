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

// --- Project & Scanning ---

export async function scanProject(path: string): Promise<ScanResult> {
  return invoke<ScanResult>('scan_project', { path })
}

export async function getScanStatus(): Promise<{ status: ScanStatus; progress: number }> {
  return invoke<{ status: ScanStatus; progress: number }>('get_scan_status')
}

export async function cancelScan(projectId: string): Promise<void> {
  return invoke<void>('cancel_scan', { projectId })
}

// --- Graph ---

export async function getGraph(projectId: string): Promise<GraphData> {
  return invoke<GraphData>('get_graph', { projectId })
}

export async function getNodeDetails(nodeId: string): Promise<FileInfo> {
  return invoke<FileInfo>('get_node_details', { nodeId })
}

export async function searchNodes(
  projectId: string,
  query: string,
  limit = 20
): Promise<GraphNode[]> {
  return invoke<GraphNode[]>('search_nodes', { projectId, query, limit })
}

export async function getDependencies(
  nodeId: string
): Promise<{ id: string; source: string; target: string; imports: string[] }[]> {
  return invoke<{ id: string; source: string; target: string; imports: string[] }[]>(
    'get_dependencies',
    { nodeId }
  )
}

export async function getDependents(
  nodeId: string
): Promise<{ id: string; source: string; target: string; imports: string[] }[]> {
  return invoke<{ id: string; source: string; target: string; imports: string[] }[]>(
    'get_dependents',
    { nodeId }
  )
}

// --- AI ---

export async function configureAI(config: AIConfig): Promise<void> {
  return invoke<void>('configure_ai', { config })
}

export async function getAIConfig(): Promise<Omit<AIConfig, 'api_key'>> {
  return invoke<Omit<AIConfig, 'api_key'>>('get_ai_config')
}

export async function explainNode(nodeId: string, symbolId?: string): Promise<NodeExplanation> {
  return invoke<NodeExplanation>('explain_node', { nodeId, symbolId })
}

export async function chat(
  projectId: string,
  message: string,
  history: { id: string; role: string; content: string; timestamp: string }[],
  contextNodeIds?: string[]
): Promise<ChatResponse> {
  return invoke<ChatResponse>('chat', { projectId, message, history, contextNodeIds })
}

export async function generateArchitectureSummary(projectId: string): Promise<{ summary: string }> {
  return invoke<{ summary: string }>('generate_architecture_summary', { projectId })
}
