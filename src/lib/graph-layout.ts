// Auto-layout using simple layered approach
// For v1: basic layered layout. Dagre can be added in v2.

import type { GraphData, GraphNode } from './types'

interface LayoutOptions {
  nodeWidth?: number
  nodeHeight?: number
  horizontalGap?: number
  verticalGap?: number
}

export function buildLayout(graphData: GraphData, options: LayoutOptions = {}): GraphData {
  const { nodeWidth = 200, nodeHeight = 80, horizontalGap = 60, verticalGap = 100 } = options

  if (!graphData.nodes.length) return graphData

  const incoming = new Map<string, string[]>()
  const outgoing = new Map<string, string[]>()

  for (const node of graphData.nodes) {
    incoming.set(node.id, [])
    outgoing.set(node.id, [])
  }

  for (const edge of graphData.edges) {
    incoming.get(edge.target)?.push(edge.source)
    outgoing.get(edge.source)?.push(edge.target)
  }

  // Compute depth via BFS from external/root nodes
  const depths = new Map<string, number>()
  const queue: string[] = []

  for (const node of graphData.nodes) {
    if (node.type === 'external' || incoming.get(node.id)?.length === 0) {
      depths.set(node.id, 0)
      queue.push(node.id)
    }
  }

  while (queue.length > 0) {
    const current = queue.shift()!
    const currentDepth = depths.get(current)!
    for (const neighbor of outgoing.get(current) ?? []) {
      if (!depths.has(neighbor)) {
        depths.set(neighbor, currentDepth + 1)
        queue.push(neighbor)
      } else {
        const existing = depths.get(neighbor) ?? Infinity
        if (currentDepth + 1 < existing) {
          depths.set(neighbor, currentDepth + 1)
        }
      }
    }
  }

  for (const node of graphData.nodes) {
    if (!depths.has(node.id)) {
      depths.set(node.id, 1)
    }
  }

  const byDepth = new Map<number, GraphNode[]>()
  for (const node of graphData.nodes) {
    const d = depths.get(node.id) ?? 1
    if (!byDepth.has(d)) byDepth.set(d, [])
    byDepth.get(d)!.push(node)
  }

  const layoutNodes = graphData.nodes.map((node: GraphNode) => {
    const depth = depths.get(node.id) ?? 1
    const nodesInSameDepth = byDepth.get(depth) ?? []
    const indexInLayer = nodesInSameDepth.indexOf(node)
    const x = depth * (nodeWidth + horizontalGap)
    const y = indexInLayer * (nodeHeight + verticalGap)
    return { ...node, position: { x, y } }
  })

  return { ...graphData, nodes: layoutNodes }
}
