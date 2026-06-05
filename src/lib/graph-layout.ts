// Auto-layout using a dependency-aware layered approach.
// The layout must stay usable for real projects, including cycles and graphs
// where import resolution produced no drawable edges.

import type { GraphData, GraphNode } from './types'

interface LayoutOptions {
  nodeWidth?: number
  nodeHeight?: number
  horizontalGap?: number
  verticalGap?: number
}

export function buildLayout(graphData: GraphData, options: LayoutOptions = {}): GraphData {
  const { nodeWidth = 200, nodeHeight = 80, horizontalGap = 80, verticalGap = 80 } = options

  if (!graphData.nodes.length) return graphData

  const nodeIds = new Set(graphData.nodes.map((node) => node.id))
  const drawableEdges = graphData.edges.filter(
    (edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)
  )

  // If there are no drawable edges, a layered dependency layout has no signal.
  // A grid is much easier to inspect than the previous single vertical stack.
  if (drawableEdges.length === 0) {
    return {
      ...graphData,
      nodes: buildGridLayout(graphData.nodes, nodeWidth, nodeHeight, horizontalGap, verticalGap),
    }
  }

  const incoming = new Map<string, string[]>()
  const outgoing = new Map<string, string[]>()

  for (const node of graphData.nodes) {
    incoming.set(node.id, [])
    outgoing.set(node.id, [])
  }

  for (const edge of drawableEdges) {
    incoming.get(edge.target)?.push(edge.source)
    outgoing.get(edge.source)?.push(edge.target)
  }

  const depths = new Map<string, number>()
  const queue: string[] = []

  const seed = (nodeId: string, depth = 0) => {
    if (depths.has(nodeId)) return
    depths.set(nodeId, depth)
    queue.push(nodeId)
  }

  // Normal roots first: external modules and nodes without incoming edges.
  for (const node of graphData.nodes) {
    if (node.type === 'external' || (incoming.get(node.id)?.length ?? 0) === 0) {
      seed(node.id, 0)
    }
  }

  // Pure cycles have no roots. Seed one stable node so traversal can still
  // assign useful depths instead of collapsing the whole SCC into one column.
  if (queue.length === 0) {
    seed(graphData.nodes[0].id, 0)
  }

  while (queue.length > 0) {
    const current = queue.shift()!
    const currentDepth = depths.get(current) ?? 0

    for (const neighbor of outgoing.get(current) ?? []) {
      if (!depths.has(neighbor)) {
        depths.set(neighbor, currentDepth + 1)
        queue.push(neighbor)
      }
    }
  }

  // Disconnected cyclic components may still be unvisited. Seed each one and
  // walk it independently so every component gets horizontal spread.
  for (const node of graphData.nodes) {
    if (depths.has(node.id)) continue

    seed(node.id, 0)
    while (queue.length > 0) {
      const current = queue.shift()!
      const currentDepth = depths.get(current) ?? 0

      for (const neighbor of outgoing.get(current) ?? []) {
        if (!depths.has(neighbor)) {
          depths.set(neighbor, currentDepth + 1)
          queue.push(neighbor)
        }
      }
    }
  }

  const byDepth = new Map<number, GraphNode[]>()
  for (const node of graphData.nodes) {
    const depth = depths.get(node.id) ?? 0
    if (!byDepth.has(depth)) byDepth.set(depth, [])
    byDepth.get(depth)!.push(node)
  }

  const layerIndex = new Map<string, number>()
  for (const [, layerNodes] of byDepth) {
    layerNodes.forEach((node, index) => layerIndex.set(node.id, index))
  }

  const layoutNodes = graphData.nodes.map((node: GraphNode) => {
    const depth = depths.get(node.id) ?? 0
    const indexInLayer = layerIndex.get(node.id) ?? 0
    const x = depth * (nodeWidth + horizontalGap)
    const y = indexInLayer * (nodeHeight + verticalGap)
    return { ...node, position: { x, y } }
  })

  return { ...graphData, nodes: layoutNodes }
}

function buildGridLayout(
  nodes: GraphNode[],
  nodeWidth: number,
  nodeHeight: number,
  horizontalGap: number,
  verticalGap: number
): GraphNode[] {
  const columns = Math.max(1, Math.ceil(Math.sqrt(nodes.length)))

  return nodes.map((node, index) => {
    const column = index % columns
    const row = Math.floor(index / columns)
    return {
      ...node,
      position: {
        x: column * (nodeWidth + horizontalGap),
        y: row * (nodeHeight + verticalGap),
      },
    }
  })
}
