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

export type OutlineItemKind =
  | 'class'
  | 'function'
  | 'method'
  | 'interface'
  | 'type'
  | 'enum'
  | 'const'
  | 'variable'
  | 'module'
  | 'field'
  | 'struct'
  | 'impl'
  | 'unknown'

export interface OutlineItem {
  id: string
  fileId: string
  name: string
  kind: OutlineItemKind
  lineStart: number
  lineEnd: number
  columnStart?: number | null
  columnEnd?: number | null
  children?: OutlineItem[]
}

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
  fileId: string
  lineStart: number
  lineEnd: number
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
  sourceFileId: string
  targetFileId: string | null
  targetModule: string | null
  imports: string[]
  isDefault: boolean
  isType: boolean
}

export interface ScanResult {
  projectId: string
  projectName: string
  rootPath: string
  filesCount: number
  symbolsCount: number
  importsCount: number
  files: FileInfo[]
  scanDurationMs: number
  status: ScanStatus
  error?: string
}

export interface GraphNode {
  id: string
  label: string
  path: string
  type: NodeType
  symbolCount: number
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
  projectId: string
  generatedAt: string
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
