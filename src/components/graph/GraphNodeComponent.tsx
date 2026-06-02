// Custom node component for React Flow
import { Handle, Position, type NodeProps } from '@xyflow/react'
import type { GraphNode } from '../../lib/types'
import { useGraphStore } from '../../stores/graphStore'

const NODE_COLORS: Record<string, string> = {
  component: '#3b82f6',
  route: '#8b5cf6',
  service: '#10b981',
  repository: '#f59e0b',
  model: '#06b6d4',
  util: '#6b7280',
  config: '#ef4444',
  test: '#84cc16',
  external: '#9ca3af',
  unknown: '#374151',
}

const NODE_LABELS: Record<string, string> = {
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

export function GraphNodeComponent({ data, selected }: NodeProps) {
  const node = data as unknown as GraphNode
  const selectNode = useGraphStore((s) => s.selectNode)
  const hoveredNodeId = useGraphStore((s) => s.hoveredNodeId)
  const setHoveredNode = useGraphStore((s) => s.setHoveredNode)

  const isHovered = hoveredNodeId === node.id
  const color = NODE_COLORS[node.type] ?? NODE_COLORS.unknown

  return (
    <div
      className={`relative rounded-lg border-2 transition-all cursor-pointer ${
        selected ? 'border-blue-400 shadow-lg' : 'border-transparent'
      } ${isHovered ? 'shadow-xl scale-105' : ''}`}
      style={{ minWidth: 160, background: '#1e293b', color: '#f8fafc' }}
      onClick={() => selectNode(node.id)}
      onMouseEnter={() => setHoveredNode(node.id)}
      onMouseLeave={() => setHoveredNode(null)}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && selectNode(node.id)}
    >
      <Handle type="target" position={Position.Top} className="!bg-slate-500" />

      {/* Header: type badge */}
      <div
        className="px-2 py-1 text-xs font-semibold rounded-t-md"
        style={{ background: color, color: '#fff' }}
      >
        {NODE_LABELS[node.type] ?? 'Unknown'}
      </div>

      {/* Body: name + path */}
      <div className="px-3 py-2 min-h-[48px]">
        <div className="font-medium text-sm truncate max-w-[160px]" title={node.label}>
          {node.label}
        </div>
        <div className="text-xs text-slate-400 truncate max-w-[160px]" title={node.path}>
          {node.path}
        </div>
        <div className="text-xs text-slate-500 mt-1">{node.symbolCount} symbols</div>
      </div>

      <Handle type="source" position={Position.Bottom} className="!bg-slate-500" />
    </div>
  )
}
