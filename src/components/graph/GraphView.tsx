// GraphView — main React Flow graph visualization
import { useCallback, useEffect, useMemo } from 'react'
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
} from '@xyflow/react'
import '@xyflow/react/dist/style.css'
import { useGraph } from '../../hooks/useGraph'
import { useGraphStore } from '../../stores/graphStore'
import { GraphNodeComponent } from './GraphNodeComponent'

const NODE_TYPES: NodeTypes = {
  graphNode: GraphNodeComponent,
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
        style: { stroke: '#475569', strokeWidth: 1.5 },
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
      <div className="flex items-center justify-center h-full text-slate-400">
        <div className="text-center">
          <div className="w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mx-auto mb-2" />
          <p>Building graph…</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-red-400">
        <p>Graph error: {error}</p>
      </div>
    )
  }

  if (!graphData) {
    return (
      <div className="flex items-center justify-center h-full text-slate-500">
        <p>No graph data. Scan a project first.</p>
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
        defaultEdgeOptions={{ type: 'smoothstep' }}
      >
        <Background color="#334155" gap={20} />
        <Controls className="!bg-slate-800 !border-slate-700 [&>button]:!bg-slate-700 [&>button]:!text-slate-300" />
        <MiniMap
          className="!bg-slate-900 !border-slate-700"
          nodeColor={(n) => {
            const d = n.data as { type?: string }
            const colors: Record<string, string> = {
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
            return colors[d.type ?? 'unknown'] ?? '#374151'
          }}
          maskColor="rgba(15,23,42,0.8)"
        />
      </ReactFlow>
    </div>
  )
}
