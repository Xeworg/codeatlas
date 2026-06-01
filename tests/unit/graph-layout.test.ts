// Vitest tests for graph layout and GraphView
import { describe, it, expect } from 'vitest'
import { buildLayout } from '../../src/lib/graph-layout'
import type { GraphData } from '../../src/lib/types'

describe('buildLayout', () => {
  it('returns empty graph unchanged', () => {
    const empty: GraphData = { nodes: [], edges: [], project_id: 'p1', generated_at: '' }
    const result = buildLayout(empty)
    expect(result.nodes).toHaveLength(0)
  })

  it('assigns positions to all nodes', () => {
    const graph: GraphData = {
      nodes: [
        { id: 'n1', label: 'A', path: 'a.ts', type: 'component', symbol_count: 2 },
        { id: 'n2', label: 'B', path: 'b.ts', type: 'service', symbol_count: 3 },
        { id: 'n3', label: 'C', path: 'c.ts', type: 'util', symbol_count: 1 },
      ],
      edges: [
        { id: 'e1', source: 'n1', target: 'n2', imports: ['B'] },
        { id: 'e2', source: 'n2', target: 'n3', imports: ['C'] },
      ],
      project_id: 'p1',
      generated_at: '',
    }
    const result = buildLayout(graph)
    expect(result.nodes).toHaveLength(3)
    result.nodes.forEach((n) => {
      expect(n.position).toBeDefined()
      expect(n.position?.x).toBeGreaterThanOrEqual(0)
      expect(n.position?.y).toBeGreaterThanOrEqual(0)
    })
  })

  it('external nodes get depth 0', () => {
    const graph: GraphData = {
      nodes: [
        {
          id: 'ext',
          label: 'react',
          path: 'node_modules/react',
          type: 'external',
          symbol_count: 0,
        },
        { id: 'a', label: 'A', path: 'a.tsx', type: 'component', symbol_count: 2 },
      ],
      edges: [{ id: 'e1', source: 'a', target: 'ext', imports: ['default'] }],
      project_id: 'p1',
      generated_at: '',
    }
    const result = buildLayout(graph)
    const extNode = result.nodes.find((n) => n.id === 'ext')
    expect(extNode?.position?.x).toBe(0)
  })
})
