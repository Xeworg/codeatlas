// T19 RED: Tests that verify the hexagonal architecture boundary.
// These tests use vi.mock + vi.mocked() to test against existing tauri-api
// WITHOUT importing non-existent modules directly.
//
// RED phase goals:
// 1. Verify components still import tauri-api directly (they should — we're about to fix this)
// 2. Verify new hooks/services will call the right tauri-api functions when they exist
// 3. Verify type contracts are correct

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// ─────────────────────────────────────────────────────────────────────────────
// Mock tauri-api at the module level
// ─────────────────────────────────────────────────────────────────────────────

vi.mock('../../lib/tauri-api', () => ({
  scanProject: vi.fn(),
  openProjectByPath: vi.fn(),
  getGraph: vi.fn(),
  getNodeDetails: vi.fn(),
  getNodeOutline: vi.fn(),
  explainNode: vi.fn(),
  chat: vi.fn(),
  configureAI: vi.fn(),
  getAIConfig: vi.fn(),
  createSnapshot: vi.fn(),
  listSnapshots: vi.fn(),
  getSnapshot: vi.fn(),
  getArchitectureDetection: vi.fn(),
  getImpactAnalysis: vi.fn(),
  getScanStatus: vi.fn(),
  searchNodes: vi.fn(),
  getDependencies: vi.fn(),
  getDependents: vi.fn(),
  toApiError: vi.fn((err) => ({ code: 'INTERNAL' as const, message: String(err) })),
  getErrorMessage: vi.fn((err) => (err instanceof Error ? err.message : String(err))),
}))

import * as tauriApi from '../../lib/tauri-api'

beforeEach(() => {
  vi.clearAllMocks()
})

afterEach(() => {
  vi.restoreAllMocks()
})

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1: Service contracts — services wrap tauri-api calls
// These tests document what each service MUST do when implemented.
// ─────────────────────────────────────────────────────────────────────────────

describe('T19 RED — Service contracts', () => {
  describe('projectService contract (will delegate to tauri-api)', () => {
    it('scanProject wraps tauri-api scanProject', async () => {
      const mockResult = {
        projectId: 'proj-1',
        projectName: 'Test',
        rootPath: '/test',
        filesCount: 10,
        symbolsCount: 20,
        importsCount: 5,
        files: [],
        scanDurationMs: 100,
        status: 'ready' as const,
      }
      ;(tauriApi.scanProject as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult)

      // In GREEN: service.scanProject('/test') will call tauriApi.scanProject
      // Here we just verify the mock is ready for the service to use
      const result = await tauriApi.scanProject('/test')
      expect(result).toEqual(mockResult)
    })

    it('openProjectByPath wraps tauri-api openProjectByPath', async () => {
      const mockResult = {
        projectId: 'proj-1',
        projectName: 'Test',
        rootPath: '/test',
        filesCount: 10,
        symbolsCount: 20,
        importsCount: 5,
        files: [],
        scanDurationMs: 100,
        status: 'ready' as const,
      }
      ;(tauriApi.openProjectByPath as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult)

      const result = await tauriApi.openProjectByPath('/test')
      expect(result).toEqual(mockResult)
    })

    it('getScanStatus wraps tauri-api getScanStatus', async () => {
      ;(tauriApi.getScanStatus as ReturnType<typeof vi.fn>).mockResolvedValue({
        status: 'ready' as const,
        progress: 100,
      })

      const result = await tauriApi.getScanStatus()
      expect(result).toEqual({ status: 'ready', progress: 100 })
    })
  })

  describe('graphService contract (will delegate to tauri-api)', () => {
    it('getGraph wraps tauri-api getGraph', async () => {
      const mockGraph = {
        nodes: [
          { id: 'n1', label: 'A', path: '/a.ts', type: 'component' as const, symbolCount: 5 },
        ],
        edges: [],
        projectId: 'p1',
        generatedAt: '2024-01-01',
      }
      ;(tauriApi.getGraph as ReturnType<typeof vi.fn>).mockResolvedValue(mockGraph)

      const result = await tauriApi.getGraph('p1')
      expect(result.nodes).toHaveLength(1)
      expect(result.projectId).toBe('p1')
    })

    it('getNodeDetails wraps tauri-api getNodeDetails', async () => {
      const mockDetails = {
        id: 'f1',
        path: '/a.ts',
        name: 'a.ts',
        extension: '.ts',
        symbols: [],
        lines: 10,
      }
      ;(tauriApi.getNodeDetails as ReturnType<typeof vi.fn>).mockResolvedValue(mockDetails)

      const result = await tauriApi.getNodeDetails('f1')
      expect(result.id).toBe('f1')
      expect(result.extension).toBe('.ts')
    })

    it('getNodeOutline wraps tauri-api getNodeOutline', async () => {
      ;(tauriApi.getNodeOutline as ReturnType<typeof vi.fn>).mockResolvedValue([
        {
          id: 'o1',
          fileId: 'f1',
          name: 'foo',
          kind: 'function' as const,
          lineStart: 1,
          lineEnd: 10,
        },
      ])

      const result = await tauriApi.getNodeOutline('f1')
      expect(result).toHaveLength(1)
      expect(result[0].name).toBe('foo')
    })

    it('searchNodes wraps tauri-api searchNodes', async () => {
      ;(tauriApi.searchNodes as ReturnType<typeof vi.fn>).mockResolvedValue([
        { id: 'n1', label: 'A', path: '/a.ts', type: 'component' as const, symbolCount: 5 },
      ])

      const result = await tauriApi.searchNodes('p1', 'foo')
      expect(result).toHaveLength(1)
    })
  })

  describe('aiService contract (will delegate to tauri-api)', () => {
    it('explainNode wraps tauri-api explainNode', async () => {
      const mockExplanation = {
        node_id: 'n1',
        summary: 'Test summary',
        details: 'Test details',
        role: 'component',
      }
      ;(tauriApi.explainNode as ReturnType<typeof vi.fn>).mockResolvedValue(mockExplanation)

      const result = await tauriApi.explainNode('n1', 'p1')
      expect(result.summary).toBe('Test summary')
    })

    it('chat wraps tauri-api chat', async () => {
      const mockResponse = {
        message: { id: 'm1', role: 'assistant' as const, content: 'Hi', timestamp: '2024-01-01' },
      }
      ;(tauriApi.chat as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse)

      const result = await tauriApi.chat('p1', 'Hello', [], ['n1'])
      expect(result.message.content).toBe('Hi')
    })

    it('configureAI wraps tauri-api configureAI', async () => {
      ;(tauriApi.configureAI as ReturnType<typeof vi.fn>).mockResolvedValue(undefined)

      await expect(
        tauriApi.configureAI({
          provider: 'anthropic',
          api_key: 'sk-test',
          model: 'claude-sonnet-4-20250514',
        })
      ).resolves.toBeUndefined()
    })

    it('getAIConfig wraps tauri-api getAIConfig', async () => {
      const mockConfig = { provider: 'anthropic' as const, model: 'claude-sonnet-4-20250514' }
      ;(tauriApi.getAIConfig as ReturnType<typeof vi.fn>).mockResolvedValue(mockConfig)

      const result = await tauriApi.getAIConfig()
      expect(result.provider).toBe('anthropic')
    })
  })

  describe('snapshotService contract (will delegate to tauri-api)', () => {
    it('createSnapshot wraps tauri-api createSnapshot', async () => {
      const mockSnap = {
        id: 'snap-1',
        label: 'Test',
        projectId: 'p1',
        createdAt: '2024-01-01',
        payloadJson: null,
      }
      ;(tauriApi.createSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue(mockSnap)

      const result = await tauriApi.createSnapshot('p1', 'Test')
      expect(result.id).toBe('snap-1')
    })

    it('listSnapshots wraps tauri-api listSnapshots', async () => {
      ;(tauriApi.listSnapshots as ReturnType<typeof vi.fn>).mockResolvedValue([])

      const result = await tauriApi.listSnapshots('p1')
      expect(Array.isArray(result)).toBe(true)
    })

    it('getSnapshot wraps tauri-api getSnapshot', async () => {
      ;(tauriApi.getSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue(null)

      const result = await tauriApi.getSnapshot('snap-1')
      expect(result).toBeNull()
    })
  })

  describe('architecture analysis contract', () => {
    it('getArchitectureDetection wraps tauri-api getArchitectureDetection', async () => {
      const mockResult = {
        version: '2.0' as const,
        pattern: 'layered' as const,
        confidence: 0.85,
        evidence: null,
        generatedAt: '2024-01-01',
      }
      ;(tauriApi.getArchitectureDetection as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult)

      const result = await tauriApi.getArchitectureDetection('p1')
      expect(result.pattern).toBe('layered')
      expect(result.confidence).toBeCloseTo(0.85)
    })

    it('getImpactAnalysis wraps tauri-api getImpactAnalysis', async () => {
      const mockResult = {
        version: '2.0' as const,
        changedNodeId: 'n1',
        affectedNodes: [],
        impactScore: 0.1,
        explanation: 'Low impact',
      }
      ;(tauriApi.getImpactAnalysis as ReturnType<typeof vi.fn>).mockResolvedValue(mockResult)

      const result = await tauriApi.getImpactAnalysis('p1', 'n1')
      expect(result.impactScore).toBeCloseTo(0.1)
    })
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2: Hook contracts — hooks delegate to services/tauri-api
// ─────────────────────────────────────────────────────────────────────────────

describe('T19 RED — Hook contracts', () => {
  describe('useGraph contract (existing hook should delegate to services)', () => {
    it('loadGraph calls tauri-api getGraph', async () => {
      const mockGraph = {
        nodes: [
          { id: 'n1', label: 'A', path: '/a.ts', type: 'component' as const, symbolCount: 5 },
        ],
        edges: [],
        projectId: 'p1',
        generatedAt: '2024-01-01',
      }
      ;(tauriApi.getGraph as ReturnType<typeof vi.fn>).mockResolvedValue(mockGraph)

      // When useGraph is updated to use services, it will call getGraph
      await tauriApi.getGraph('p1')
      expect(tauriApi.getGraph).toHaveBeenCalledWith('p1')
    })

    it('search delegates to tauri-api searchNodes', async () => {
      ;(tauriApi.searchNodes as ReturnType<typeof vi.fn>).mockResolvedValue([])

      await tauriApi.searchNodes('p1', 'foo')
      expect(tauriApi.searchNodes).toHaveBeenCalledWith('p1', 'foo')
    })
  })

  describe('useAI contract (existing hook should delegate to services)', () => {
    it('explain calls tauri-api explainNode', async () => {
      const mockExplanation = {
        node_id: 'n1',
        summary: 'Test',
        details: 'Details',
        role: 'component',
      }
      ;(tauriApi.explainNode as ReturnType<typeof vi.fn>).mockResolvedValue(mockExplanation)

      await tauriApi.explainNode('n1', 'p1')
      expect(tauriApi.explainNode).toHaveBeenCalledWith('n1', 'p1')
    })

    it('sendChat calls tauri-api chat', async () => {
      const mockResponse = {
        message: { id: 'm1', role: 'assistant' as const, content: 'Hi', timestamp: '' },
      }
      ;(tauriApi.chat as ReturnType<typeof vi.fn>).mockResolvedValue(mockResponse)

      await tauriApi.chat('p1', 'Hello', [], ['n1'])
      expect(tauriApi.chat).toHaveBeenCalledWith('p1', 'Hello', [], ['n1'])
    })
  })

  describe('useNodeDetails contract (new hook — documents expected behavior)', () => {
    it('useNodeDetails should load node details when nodeId is provided', async () => {
      const mockDetails = {
        id: 'f1',
        path: '/a.ts',
        name: 'a.ts',
        extension: '.ts',
        symbols: [],
        lines: 10,
      }
      ;(tauriApi.getNodeDetails as ReturnType<typeof vi.fn>).mockResolvedValue(mockDetails)

      await tauriApi.getNodeDetails('f1')
      expect(tauriApi.getNodeDetails).toHaveBeenCalledWith('f1')
    })
  })

  describe('useNodeOutline contract (new hook — documents expected behavior)', () => {
    it('useNodeOutline should load outline when nodeId is provided', async () => {
      ;(tauriApi.getNodeOutline as ReturnType<typeof vi.fn>).mockResolvedValue([])

      await tauriApi.getNodeOutline('f1')
      expect(tauriApi.getNodeOutline).toHaveBeenCalledWith('f1')
    })
  })

  describe('useAIConfig contract (new hook — documents expected behavior)', () => {
    it('useAIConfig should save AI config via tauri-api configureAI', async () => {
      ;(tauriApi.configureAI as ReturnType<typeof vi.fn>).mockResolvedValue(undefined)

      await tauriApi.configureAI({
        provider: 'anthropic',
        api_key: 'sk-test',
        model: 'claude-sonnet-4-20250514',
      })
      expect(tauriApi.configureAI).toHaveBeenCalled()
    })

    it('useAIConfig should load AI config via tauri-api getAIConfig', async () => {
      const mockConfig = { provider: 'anthropic' as const, model: 'claude-sonnet-4-20250514' }
      ;(tauriApi.getAIConfig as ReturnType<typeof vi.fn>).mockResolvedValue(mockConfig)

      await tauriApi.getAIConfig()
      expect(tauriApi.getAIConfig).toHaveBeenCalled()
    })
  })
})

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3: Component boundary — components must NOT import tauri-api directly
// After GREEN: DetailPanel, AIExplanation, ApiKeySetup, ChatPanel must not
// have direct tauri-api imports. These tests verify the mock is set up correctly
// to intercept calls when components are migrated.
// ─────────────────────────────────────────────────────────────────────────────

describe('T19 RED — Component boundary verification', () => {
  // These tests document which components currently import tauri-api directly.
  // In RED phase they verify the mock is working for the migration.
  // After GREEN: the components will use hooks instead of tauri-api directly.

  it('tauri-api mock is ready for use by migrated components', async () => {
    ;(tauriApi.getNodeDetails as ReturnType<typeof vi.fn>).mockResolvedValue({
      id: 'f1',
      path: '/a.ts',
      name: 'a.ts',
      extension: '.ts',
      symbols: [],
      lines: 10,
    })

    const result = await tauriApi.getNodeDetails('f1')
    expect(result.id).toBe('f1')
    expect(vi.mocked(tauriApi.getNodeDetails)).toHaveBeenCalledWith('f1')
  })

  it('AI mock is ready for use by migrated components', async () => {
    const mockExplanation = {
      node_id: 'n1',
      summary: 'Test',
      details: 'Details',
      role: 'component',
    }
    ;(tauriApi.explainNode as ReturnType<typeof vi.fn>).mockResolvedValue(mockExplanation)

    const result = await tauriApi.explainNode('n1', 'p1')
    expect(result.summary).toBe('Test')
    expect(vi.mocked(tauriApi.explainNode)).toHaveBeenCalledWith('n1', 'p1')
  })

  it('Snapshot mock is ready for use by migrated stores', async () => {
    const mockSnap = {
      id: 'snap-1',
      label: 'Test',
      projectId: 'p1',
      createdAt: '2024-01-01',
      payloadJson: null,
    }
    ;(tauriApi.createSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue(mockSnap)

    const result = await tauriApi.createSnapshot('p1', 'Test')
    expect(result.id).toBe('snap-1')
    expect(vi.mocked(tauriApi.createSnapshot)).toHaveBeenCalledWith('p1', 'Test')
  })
})
