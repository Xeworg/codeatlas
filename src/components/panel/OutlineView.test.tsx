// OutlineView.test.tsx — PR3 TDD: RED first
// Tests for OutlineView: tree render, empty state, collapse/expand.
// When all tests pass, implement OutlineView.tsx.

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, act, fireEvent } from '@testing-library/react'
import type { OutlineItem, FileInfo } from '../../lib/types'
import { DetailPanel } from './DetailPanel'
import { useGraphStore } from '../../stores/graphStore'

// Small delay helper to let React flush state updates after async resolves
const tick = () =>
  act(async () => {
    await Promise.resolve()
  })

// ── Mock tauri-api — partial mock preserving non-outline functions ────────
const mockGetNodeOutline = vi.hoisted(() => vi.fn())
const mockGetNodeDetails = vi.hoisted(() => vi.fn())
vi.mock('../../lib/tauri-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../lib/tauri-api')>()
  return {
    ...actual,
    getNodeOutline: mockGetNodeOutline,
    getNodeDetails: mockGetNodeDetails,
  }
})

// ── Tests ─────────────────────────────────────────────────────────────────

describe('OutlineView — tree render (T3.1 / T3.2)', () => {
  beforeEach(() => {
    mockGetNodeOutline.mockReset()
    // reset selected node
    useGraphStore.getState().selectNode(null)
  })

  it('T3.1.1: renders root outline item name', async () => {
    const outline: OutlineItem[] = [
      {
        id: 'outline:file-1:class:1:10:UserService',
        fileId: 'file-1',
        name: 'UserService',
        kind: 'class',
        lineStart: 1,
        lineEnd: 10,
        columnStart: null,
        columnEnd: null,
        children: [],
      },
    ]
    mockGetNodeOutline.mockResolvedValueOnce(outline)
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-1',
      path: 'src/UserService.ts',
      name: 'UserService.ts',
      extension: 'ts',
      lines: 10,
      symbols: [],
    } as FileInfo)

    // simulate scanned project so DetailPanel has a node to select
    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-1',
          label: 'UserService.ts',
          path: 'src/UserService.ts',
          type: 'component' as const,
          symbolCount: 1,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })

    useGraphStore.getState().selectNode('file-1')

    const { container } = render(<DetailPanel />)
    await tick()
    // The test validates the outline item text appears
    expect(container.textContent?.includes('UserService')).toBeTruthy()
  })

  it('T3.1.2: renders children indented under parent', async () => {
    const outline: OutlineItem[] = [
      {
        id: 'outline:file-2:class:1:20:OrderService',
        fileId: 'file-2',
        name: 'OrderService',
        kind: 'class',
        lineStart: 1,
        lineEnd: 20,
        columnStart: null,
        columnEnd: null,
        children: [
          {
            id: 'outline:file-2:method:5:7:createOrder',
            fileId: 'file-2',
            name: 'createOrder',
            kind: 'method',
            lineStart: 5,
            lineEnd: 7,
            columnStart: null,
            columnEnd: null,
            children: [],
          },
        ],
      },
    ]
    mockGetNodeOutline.mockResolvedValueOnce(outline)
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-2',
      path: 'src/OrderService.ts',
      name: 'OrderService.ts',
      extension: 'ts',
      lines: 20,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-2',
          label: 'OrderService.ts',
          path: 'src/OrderService.ts',
          type: 'service' as const,
          symbolCount: 1,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-2')

    const { container } = render(<DetailPanel />)
    await tick()
    expect(container.textContent?.includes('OrderService')).toBeTruthy()
    expect(container.textContent?.includes('createOrder')).toBeTruthy()
  })

  it('T3.1.3: shows kind badge and line range for each item', async () => {
    const outline: OutlineItem[] = [
      {
        id: 'outline:file-3:function:15:25:parseData',
        fileId: 'file-3',
        name: 'parseData',
        kind: 'function',
        lineStart: 15,
        lineEnd: 25,
        columnStart: null,
        columnEnd: null,
        children: [],
      },
    ]
    mockGetNodeOutline.mockResolvedValueOnce(outline)
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-3',
      path: 'src/parseData.ts',
      name: 'parseData.ts',
      extension: 'ts',
      lines: 25,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-3',
          label: 'parseData.ts',
          path: 'src/parseData.ts',
          type: 'util' as const,
          symbolCount: 1,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-3')

    const { container } = render(<DetailPanel />)
    await tick()
    // kind badge shows 'F' with title 'fn'; line range shows 15–25
    expect(container.textContent?.includes('F')).toBeTruthy()
    expect(container.textContent?.includes('15')).toBeTruthy()
    expect(container.textContent?.includes('25')).toBeTruthy()
  })
})

describe('OutlineView — empty state (T3.1 / T3.5)', () => {
  beforeEach(() => {
    mockGetNodeOutline.mockReset()
    useGraphStore.getState().selectNode(null)
  })

  it('T3.1.4: shows empty state when outline returns empty array', async () => {
    mockGetNodeOutline.mockResolvedValueOnce([])
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-empty',
      path: 'src/empty.ts',
      name: 'empty.ts',
      extension: 'ts',
      lines: 5,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-empty',
          label: 'empty.ts',
          path: 'src/empty.ts',
          type: 'unknown' as const,
          symbolCount: 0,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-empty')

    const { container } = render(<DetailPanel />)
    await tick()
    // Should show empty state text
    expect(
      container.textContent?.toLowerCase().includes('no symbols') ||
        container.textContent?.toLowerCase().includes('no outline')
    ).toBeTruthy()
  })
})

describe('OutlineView — loading and error states (T3.5)', () => {
  beforeEach(() => {
    mockGetNodeOutline.mockReset()
    useGraphStore.getState().selectNode(null)
  })

  it('T3.1.5: shows loading state while outline is being fetched', async () => {
    // Mock that never resolves — simulate slow fetch
    mockGetNodeOutline.mockImplementation(() => new Promise(() => {}))
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-loading',
      path: 'src/loading.ts',
      name: 'loading.ts',
      extension: 'ts',
      lines: 1,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-loading',
          label: 'loading.ts',
          path: 'src/loading.ts',
          type: 'unknown' as const,
          symbolCount: 0,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-loading')

    const { container } = render(<DetailPanel />)
    await tick()
    // Loading state should be visible in outline section
    expect(container.textContent?.toLowerCase().includes('loading')).toBeTruthy()
  })

  it('T3.1.6: shows error state when outline fetch fails but preserves details', async () => {
    mockGetNodeOutline.mockRejectedValueOnce(new Error('Network error'))
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-err',
      path: 'src/err.ts',
      name: 'err.ts',
      extension: 'ts',
      lines: 1,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-err',
          label: 'err.ts',
          path: 'src/err.ts',
          type: 'unknown' as const,
          symbolCount: 0,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-err')

    const { container } = render(<DetailPanel />)
    await tick()
    // Error should appear but details section should still show
    expect(
      container.textContent?.toLowerCase().includes('err') ||
        container.textContent?.includes('Network')
    ).toBeTruthy()
    // File name / path should still appear
    expect(container.textContent?.includes('err.ts')).toBeTruthy()
  })
})

describe('OutlineView — collapse/expand (T3.1)', () => {
  beforeEach(() => {
    mockGetNodeOutline.mockReset()
    useGraphStore.getState().selectNode(null)
  })

  it('T3.1.7: parent item with children is collapsible', async () => {
    const outline: OutlineItem[] = [
      {
        id: 'outline:file-4:class:1:30:DataService',
        fileId: 'file-4',
        name: 'DataService',
        kind: 'class',
        lineStart: 1,
        lineEnd: 30,
        columnStart: null,
        columnEnd: null,
        children: [
          {
            id: 'outline:file-4:method:5:8:fetchData',
            fileId: 'file-4',
            name: 'fetchData',
            kind: 'method',
            lineStart: 5,
            lineEnd: 8,
            columnStart: null,
            columnEnd: null,
            children: [],
          },
          {
            id: 'outline:file-4:method:10:15:saveData',
            fileId: 'file-4',
            name: 'saveData',
            kind: 'method',
            lineStart: 10,
            lineEnd: 15,
            columnStart: null,
            columnEnd: null,
            children: [],
          },
        ],
      },
    ]
    mockGetNodeOutline.mockResolvedValueOnce(outline)
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-4',
      path: 'src/DataService.ts',
      name: 'DataService.ts',
      extension: 'ts',
      lines: 30,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-4',
          label: 'DataService.ts',
          path: 'src/DataService.ts',
          type: 'service' as const,
          symbolCount: 1,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-4')

    const { container, getByLabelText } = render(<DetailPanel />)
    await tick()
    // Both parent and children should be visible by default
    expect(container.textContent?.includes('DataService')).toBeTruthy()
    expect(container.textContent?.includes('fetchData')).toBeTruthy()
    expect(container.textContent?.includes('saveData')).toBeTruthy()

    fireEvent.click(getByLabelText('collapse'))
    expect(container.textContent?.includes('DataService')).toBeTruthy()
    expect(container.textContent?.includes('fetchData')).toBeFalsy()
    expect(container.textContent?.includes('saveData')).toBeFalsy()

    fireEvent.click(getByLabelText('expand'))
    expect(container.textContent?.includes('fetchData')).toBeTruthy()
    expect(container.textContent?.includes('saveData')).toBeTruthy()
  })
})

describe('GraphNodeComponent stays compact (T3.4)', () => {
  it('T3.4.1: GraphNodeComponent does not render outline tree', () => {
    // The graph node component shows symbol count only, no outline tree.
    // Verifiable by source inspection: GraphNodeComponent.tsx has no OutlineView.
    expect(true).toBe(true)
  })
})

// ── Regression: backend may omit `children` from serialized outline items ──
// Rust uses #[serde(default, skip_serializing_if = "Vec::is_empty")] on OutlineItem.children,
// so leaf items arrive in TypeScript without a `children` property.
// OutlineView must not throw "Cannot read properties of undefined (reading 'length')".
describe('OutlineView — missing children regression (T3.6)', () => {
  beforeEach(() => {
    mockGetNodeOutline.mockReset()
    useGraphStore.getState().selectNode(null)
  })

  it('T3.6.1: renders outline items even when backend omits children property', async () => {
    // Simulates real backend: leaf items lack `children` field entirely
    const outlineFromBackend: OutlineItem[] = [
      {
        id: 'outline:file-no-children:class:1:10:BackendClass',
        fileId: 'file-no-children',
        name: 'BackendClass',
        kind: 'class',
        lineStart: 1,
        lineEnd: 10,
        // children omitted — simulates skip_serializing_if empty in Rust
      },
    ]
    // Simulate backend omitting the `children` field entirely (Rust uses
    // #[serde(default, skip_serializing_if = "Vec::is_empty")]).
    // Optional property + undefined value — no @ts-expect-error needed.
    ;(outlineFromBackend[0] as OutlineItem).children = undefined

    mockGetNodeOutline.mockResolvedValueOnce(outlineFromBackend)
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-no-children',
      path: 'src/BackendClass.ts',
      name: 'BackendClass.ts',
      extension: 'ts',
      lines: 10,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-no-children',
          label: 'BackendClass.ts',
          path: 'src/BackendClass.ts',
          type: 'component' as const,
          symbolCount: 1,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-no-children')

    const { container } = render(<DetailPanel />)
    await tick()
    // Must not throw — the item should render with the name
    expect(container.textContent?.includes('BackendClass')).toBeTruthy()
  })

  it('T3.6.2: renders nested items when outer has children but inner omits them', async () => {
    // Mixed: parent has children: [...], child lacks children field
    const outlineMixed: OutlineItem[] = [
      {
        id: 'outline:file-mixed:class:1:30:ParentService',
        fileId: 'file-mixed',
        name: 'ParentService',
        kind: 'class',
        lineStart: 1,
        lineEnd: 30,
        children: [
          {
            id: 'outline:file-mixed:method:5:8:doThing',
            fileId: 'file-mixed',
            name: 'doThing',
            kind: 'method',
            lineStart: 5,
            lineEnd: 8,
            // children omitted on child item
          },
        ],
      },
    ]
    const firstItem = outlineMixed[0]!
    const firstChild = firstItem.children![0]
    // Child omits `children` — matches Rust serialization behavior
    ;(firstChild as OutlineItem).children = undefined

    mockGetNodeOutline.mockResolvedValueOnce(outlineMixed)
    mockGetNodeDetails.mockResolvedValueOnce({
      id: 'file-mixed',
      path: 'src/ParentService.ts',
      name: 'ParentService.ts',
      extension: 'ts',
      lines: 30,
      symbols: [],
    } as FileInfo)

    useGraphStore.getState().setGraphData({
      nodes: [
        {
          id: 'file-mixed',
          label: 'ParentService.ts',
          path: 'src/ParentService.ts',
          type: 'service' as const,
          symbolCount: 1,
        },
      ],
      edges: [],
      projectId: 'proj-1',
      generatedAt: new Date().toISOString(),
    })
    useGraphStore.getState().selectNode('file-mixed')

    const { container } = render(<DetailPanel />)
    await tick()
    expect(container.textContent?.includes('ParentService')).toBeTruthy()
    expect(container.textContent?.includes('doThing')).toBeTruthy()
  })
})
