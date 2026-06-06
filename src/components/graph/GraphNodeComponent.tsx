// GraphNodeComponent — React Flow custom node
// Slice 5 (milestone 2): redesigned to match the reference's
// rounded dark cards with a colored left border accent and a
// per-type Lucide icon. Replaces the full-width colored header
// bar with a much subtler indicator that still preserves the
// type color signal. Layout dimensions (min/max width, min
// body height) are preserved so React Flow's dagre layout
// stays stable.
import { Handle, Position, type NodeProps } from '@xyflow/react'
import {
  Box,
  Route,
  Server,
  Database,
  FileCode,
  Cog,
  FlaskConical,
  Globe,
  HelpCircle,
  Layers,
  Link2,
  type LucideIcon,
} from 'lucide-react'
import type { GraphNode, NodeType } from '../../lib/types'
import { useGraphStore } from '../../stores/graphStore'

// Muted variant of the original color map — the reference uses
// these as a thin accent, not a saturated solid block.
const NODE_ACCENTS: Record<NodeType, string> = {
  component: '#3b82f6',
  route: '#10b981',
  service: '#f59e0b',
  repository: '#8b5cf6',
  model: '#06b6d4',
  util: '#6b7280',
  config: '#ef4444',
  test: '#84cc16',
  external: '#9ca3af',
  unknown: '#374151',
}

const NODE_LABELS: Record<NodeType, string> = {
  component: 'Component',
  route: 'Route',
  service: 'Service',
  repository: 'Repository',
  model: 'Model',
  util: 'Util',
  config: 'Config',
  test: 'Test',
  external: 'External',
  unknown: 'Unknown',
}

const NODE_ICONS: Record<NodeType, LucideIcon> = {
  component: Box,
  route: Route,
  service: Server,
  repository: Database,
  model: Layers,
  util: FileCode,
  config: Cog,
  test: FlaskConical,
  external: Globe,
  unknown: HelpCircle,
}

export function GraphNodeComponent({ data, selected }: NodeProps) {
  const node = data as unknown as GraphNode
  const selectNode = useGraphStore((s) => s.selectNode)
  const hoveredNodeId = useGraphStore((s) => s.hoveredNodeId)
  const setHoveredNode = useGraphStore((s) => s.setHoveredNode)

  const isHovered = hoveredNodeId === node.id
  const type = (node.type ?? 'unknown') as NodeType
  const accent = NODE_ACCENTS[type] ?? NODE_ACCENTS.unknown
  const label = NODE_LABELS[type] ?? 'Unknown'
  const TypeIcon = NODE_ICONS[type] ?? NODE_ICONS.unknown

  return (
    <div
      className={`group relative rounded-md overflow-hidden transition-all cursor-pointer bg-surface-elevated text-text-primary border ${
        selected
          ? 'border-accent-secondary shadow-elevated'
          : isHovered
            ? 'border-border-strong shadow-panel'
            : 'border-border-subtle shadow-panel'
      }`}
      style={{ minWidth: 200, maxWidth: 240 }}
      onClick={() => selectNode(node.id)}
      onMouseEnter={() => setHoveredNode(node.id)}
      onMouseLeave={() => setHoveredNode(null)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && selectNode(node.id)}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!bg-text-muted !border-surface-elevated"
      />

      {/* Left color accent — single thin band, no full-width header bar */}
      <div
        aria-hidden
        className="absolute inset-y-0 left-0 w-1"
        style={{ background: accent }}
      />

      {/* Body */}
      <div className="pl-3 pr-3 py-2 min-h-[68px] flex flex-col gap-1">
        {/* Type row */}
        <div className="flex items-center gap-1.5">
          <span
            className="inline-flex items-center justify-center w-4 h-4 rounded-sm"
            style={{ color: accent }}
            aria-hidden
          >
            <TypeIcon size={12} strokeWidth={1.75} />
          </span>
          <span
            className="text-[10px] font-semibold uppercase tracking-wider"
            style={{ color: accent }}
          >
            {label}
          </span>
        </div>

        {/* Name + path */}
        <div className="font-semibold text-sm text-text-primary truncate" title={node.label}>
          {node.label}
        </div>
        <div
          className="text-[11px] font-mono text-text-muted truncate"
          title={node.path}
        >
          {node.path}
        </div>

        {/* Footer */}
        <div className="flex items-center gap-1 text-[11px] text-text-muted">
          <Link2 size={10} strokeWidth={1.75} aria-hidden />
          <span>{node.symbolCount} symbols</span>
        </div>
      </div>

      <Handle
        type="source"
        position={Position.Right}
        className="!bg-text-muted !border-surface-elevated"
      />
    </div>
  )
}
