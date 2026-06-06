// GraphView — main React Flow graph visualization
// Slice 5 (milestone 2): re-skinned to the dark reference palette.
// The background grid, controls, and minimap now use the surface
// tokens. Loading / error / empty states are routed through the
// restyled common components. The colored node palette is
// aligned with the new accent behavior.
import { useCallback, useEffect, useMemo, type CSSProperties } from 'react'
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
  type NodeTypes,
  type ReactFlowProps,
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { Network, AlertOctagon } from 'lucide-react'
import { useGraph } from '../../hooks/useGraph'
import { useGraphStore } from '../../stores/graphStore'
import { GraphNodeComponent } from './GraphNodeComponent'
import { Spinner } from '../common/Spinner'
import { ErrorState } from '../common/ErrorState'
import { EmptyState } from '../common/EmptyState'
import type { NodeType } from '../../lib/types'

const NODE_TYPES: NodeTypes = {
  graphNode: GraphNodeComponent,
}

const NODE_MINIMAP_COLORS: Record<NodeType, string> = {
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

// React Flow prop overrides aligned with the reference's chrome
const reactFlowStyle: CSSProperties = {
  background: 'var(--surface-base, #0a0a0b)',
}
const backgroundColor = 'rgba(255, 255, 255, 0.05)'
const controlsClassName =
  '!bg-surface-elevated !border-border-subtle !shadow-panel [&>button]:!bg-surface-elevated [&>button]:!border-border-subtle [&>button]:!text-text-secondary [&>button:hover]:!bg-surface-hover [&>button:hover]:!text-text-primary [&>button>svg]:!fill-current'
const minimapClassName = '!bg-surface-elevated !border !border-border-subtle !shadow-panel'
const minimapMaskColor = 'rgba(10, 10, 11, 0.78)'

const defaultEdgeOptions: ReactFlowProps['defaultEdgeOptions'] = {
  type: 'smoothstep',
}

export function GraphView() {
  const { graphData, isLoading, error } = useGraph()
  const selectedNodeId = useGraphStore((s) => s.selectedNodeId)
  const hoveredNodeId = useGraphStore((s) => s.hoveredNodeId)
  const selectNode = useGraphStore((s) => s.selectNode)

  const flowNodes = useMemo(() => {
    if (!graphData) return []
    return graphData.nodes.map(
      (n): Node => ({
        id: n.id,
        type: 'graphNode',
        position: n.position ?? { x: 0, y: 0 },
        data: n as unknown as Record<string, unknown>,
        selected: n.id === selectedNodeId,
      })
    )
  }, [graphData, selectedNodeId])

  const flowEdges = useMemo(() => {
    if (!graphData) return []
    return graphData.edges.map(
      (e): Edge => ({
        id: e.id,
        source: e.source,
        target: e.target,
        animated: hoveredNodeId === e.source || hoveredNodeId === e.target,
        style: { stroke: '#3a3a42', strokeWidth: 1.5 },
      })
    )
  }, [graphData, hoveredNodeId])

  const [nodes, setNodes, onNodesChange] = useNodesState(flowNodes)
  const [edges, setEdges, onEdgesChange] = useEdgesState(flowEdges)

  useEffect(() => {
    setNodes(flowNodes)
    setEdges(flowEdges)
  }, [flowNodes, flowEdges, setNodes, setEdges])

  const onNodeClick = useCallback(
    (_event: unknown, node: Node) => {
      selectNode(node.id)
    },
    [selectNode]
  )

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Spinner size="lg" label="Building graph" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <ErrorState
          icon={<AlertOctagon size={24} strokeWidth={1.75} />}
          message={error}
        />
      </div>
    )
  }

  if (!graphData) {
    return (
      <div className="flex items-center justify-center h-full">
        <EmptyState
          icon={<Network size={26} strokeWidth={1.5} />}
          title="No graph data"
          description="Scan a project to explore its architecture"
        />
      </div>
    )
  }

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        nodeTypes={NODE_TYPES}
        fitView
        minZoom={0.1}
        maxZoom={2}
        defaultEdgeOptions={defaultEdgeOptions}
        style={reactFlowStyle}
        proOptions={{ hideAttribution: true }}
      >
        <Background color={backgroundColor} gap={20} size={1} />
        <Controls className={controlsClassName} showInteractive={false} />
        <MiniMap
          className={minimapClassName}
          maskColor={minimapMaskColor}
          nodeColor={(n) => {
            const d = n.data as { type?: NodeType }
            return NODE_MINIMAP_COLORS[d.type ?? 'unknown'] ?? NODE_MINIMAP_COLORS.unknown
          }}
          nodeStrokeColor="rgba(255,255,255,0.05)"
          pannable
          zoomable
        />
      </ReactFlow>
    </div>
  )
}
