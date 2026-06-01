// Domain models for CodeAtlas v1
// These types are the canonical contract between Rust backend and TypeScript frontend.
// All imports in models/ must be std or serde only.

export type ScanStatus = 'idle' | 'scanning' | 'building_graph' | 'ready' | 'error'

export type SymbolKind =
  | 'class'
  | 'function'
  | 'arrow_function'
  | 'method'
  | 'interface'
  | 'type_alias'
  | 'enum'
  | 'variable'
  | 'const'
  | 'struct'
  | 'impl'
  | 'unknown'

export type NodeType =
  | 'component'
  | 'route'
  | 'service'
  | 'repository'
  | 'model'
  | 'util'
  | 'config'
  | 'test'
  | 'external'
  | 'unknown'

export interface SymbolInfo {
  id: string
  name: string
  kind: SymbolKind
  file_id: string
  line_start: number
  line_end: number
  exports: boolean
}

export interface FileInfo {
  id: string
  path: string
  name: string
  extension: string
  symbols: SymbolInfo[]
  lines: number
}

export interface ImportInfo {
  id: string
  source_file_id: string
  target_file_id: string | null
  target_module: string | null
  imports: string[]
  is_default: boolean
  is_type: boolean
}

export interface ScanResult {
  project_id: string
  project_name: string
  root_path: string
  files_count: number
  symbols_count: number
  imports_count: number
  files: FileInfo[]
  scan_duration_ms: number
  status: ScanStatus
  error?: string
}

export interface GraphNode {
  id: string
  label: string
  path: string
  type: NodeType
  symbol_count: number
  position?: { x: number; y: number }
}

export interface GraphEdge {
  id: string
  source: string
  target: string
  imports: string[]
}

export interface GraphData {
  nodes: GraphNode[]
  edges: GraphEdge[]
  project_id: string
  generated_at: string
}

export interface NodeExplanation {
  node_id: string
  summary: string
  details: string
  dependencies_note?: string
  role: string
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: string
}

export interface ChatResponse {
  message: ChatMessage
  referenced_nodes?: string[]
}

export interface AIConfig {
  provider: 'anthropic' | 'custom'
  api_key: string
  model: string
  endpoint?: string
}

export interface ApiError {
  code: ErrorCode
  message: string
  details?: Record<string, unknown>
}

export type ErrorCode =
  | 'PATH_NOT_FOUND'
  | 'ACCESS_DENIED'
  | 'SCAN_TIMEOUT'
  | 'INVALID_KEY'
  | 'UNREACHABLE'
  | 'RATE_LIMITED'
  | 'TOKEN_LIMIT'
  | 'INTERNAL'

// ===========================================================================
// v2 Contract Types
// ===========================================================================
// Aligned with openspec/changes/v2-advanced-analysis/design.md §5
// These types extend the v1 contract surface without modifying v1 fields.

export type ArchitecturePattern = 'mvc' | 'layered' | 'clean' | 'hexagonal' | 'unknown'

export interface ArchitectureDetectionResult {
  version: '2.0'
  pattern: ArchitecturePattern
  confidence: number // 0..1
  evidence: {
    nodes: string[]
    edges: Array<{ source: string; target: string; kind: string }>
    reasons: string[]
  } | null
  generatedAt: string
}

export interface ImpactAnalysisResult {
  version: '2.0'
  changedNodeId: string
  affectedNodes: string[]
  impactScore: number // 0..1
  explanation: string
}

export interface GraphInsights {
  version: '2.0'
  cycles: Array<{ nodes: string[]; length: number }>
  hotspots: Array<{ nodeId: string; couplingScore: number; reason: string }>
  avgCoupling: number | null
  density: number | null
  status?: 'ok' | 'timeout' | 'error'
}

export interface ExportPayload {
  version: '2.0'
  format: 'json' | 'png'
  graphData: unknown
  insights: GraphInsights | null
  metadata: { projectId: string; generatedAt: string }
}
