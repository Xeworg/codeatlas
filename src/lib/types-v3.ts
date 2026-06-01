// ===========================================================================
// v3 Contract Types — Workspace, Snapshots, Collaboration
// Aligned with openspec/changes/v3-collaboration-platform/proposal.md
// These types extend v1+v2 contracts without modifying existing fields.
// ===========================================================================

// MARK: PR1 — Workspace domain

export interface Workspace {
  id: string
  name: string
  createdAt: string
}

export interface WorkspaceProject {
  workspaceId: string
  projectId: string
}

export interface Snapshot {
  id: string
  projectId: string
  workspaceId?: string
  label: string
  createdAt: string
  payloadJson?: string
}

// MARK: PR5 — Snapshot capture (payload shape)
// Fleshed out in PR5; stub here so tests can reference it.

export interface SnapshotPayload {
  nodes: unknown[]
  edges: unknown[]
  insights?: unknown
  architectureDetection?: unknown
}

// MARK: H2 — Annotations

export interface Comment {
  id: string
  projectId: string
  nodeId: string
  author: string
  kind: 'comment' | 'todo' | 'review' | 'issue'
  text: string
  createdAt: string
}

// Placeholder for H3 — fleshed out in PR7
export interface HealthScoreTimeline {
  records: HealthRecord[]
  projectId: string
  from: string
  to: string
}

export interface HealthRecord {
  id: string
  recordedAt: string
  overallScore: number // 0..100
  couplingScore: number
  complexityScore: number
  cycleCount: number
  hotspotCount: number
}

// Placeholder for H3 — fleshed out in PR8
export interface ExecutiveArchitectureSummary {
  workspaceId: string
  totalProjects: number
  totalFiles: number
  avgHealthScore: number | null
  trend: 'up' | 'down' | 'stable'
  topHotspots: Array<{ nodeId: string; couplingScore: number }>
  generatedAt: string
}

// Placeholder for H3 — fleshed out in PR8
export interface SnapshotDiffPayload {
  baseSnapshotId: string
  targetSnapshotId: string
  nodesAdded: string[]
  nodesRemoved: string[]
  nodesModified: string[]
  edgesAdded: string[]
  edgesRemoved: string[]
  couplingDelta: number
  complexityDelta: number
  cyclesDelta: number
}

// Placeholder for H3 — fleshed out in PR8
export type C4Level = 1 | 2

export interface C4ViewPayload {
  level: C4Level
  systems?: string[]
  containers?: string[]
  warning?: string
}
