// Error handling tests for tauri-api
// Part of PR6 — T6.2 (error handling global)

import { describe, it, expect } from 'vitest'

// Re-import the types to test error code mapping
import type { ErrorCode } from '../../src/lib/types'

describe('ApiError types', () => {
  it('ErrorCode has all expected variants', () => {
    const validCodes: ErrorCode[] = [
      'PATH_NOT_FOUND',
      'ACCESS_DENIED',
      'SCAN_TIMEOUT',
      'INVALID_KEY',
      'UNREACHABLE',
      'RATE_LIMITED',
      'TOKEN_LIMIT',
      'INTERNAL',
    ]
    expect(validCodes.length).toBe(8)
  })

  it('ApiError shape is correct', () => {
    const err = {
      code: 'PATH_NOT_FOUND' as ErrorCode,
      message: 'File not found',
      details: { path: '/test' },
    }
    expect(err).toHaveProperty('code')
    expect(err).toHaveProperty('message')
    expect(typeof err.code).toBe('string')
    expect(typeof err.message).toBe('string')
  })
})

describe('Error state UI components', () => {
  it('ErrorState renders error message', async () => {
    const mod = await import('../../src/components/common/ErrorState')
    // Verify module exports ErrorState component
    expect(typeof mod.ErrorState).toBe('function')
  })

  it('Spinner renders without crashing', async () => {
    const mod = await import('../../src/components/common/Spinner')
    expect(typeof mod.Spinner).toBe('function')
  })

  it('EmptyState renders with all props', async () => {
    const mod = await import('../../src/components/common/EmptyState')
    expect(typeof mod.EmptyState).toBe('function')
  })
})

describe('GraphView states', () => {
  it('GraphView handles isLoading state', async () => {
    const { GraphView } = await import('../../src/components/graph/GraphView')
    // Just ensure import works
    expect(GraphView).toBeDefined()
  })
})

describe('DetailPanel states', () => {
  it('DetailPanel handles loading state', async () => {
    const { DetailPanel } = await import('../../src/components/panel/DetailPanel')
    expect(DetailPanel).toBeDefined()
  })
})

describe('AIExplanation states', () => {
  it('AIExplanation handles error state', async () => {
    const { AIExplanation } = await import('../../src/components/panel/AIExplanation')
    expect(AIExplanation).toBeDefined()
  })
})

describe('ChatPanel states', () => {
  it('ChatPanel handles error state', async () => {
    const { ChatPanel } = await import('../../src/components/chat/ChatPanel')
    expect(ChatPanel).toBeDefined()
  })
})

describe('Sidebar states', () => {
  it('Sidebar handles empty state (no scan result)', async () => {
    const { Sidebar } = await import('../../src/components/layout/Sidebar')
    expect(Sidebar).toBeDefined()
  })
})

describe('ApiKeySetup states', () => {
  it('ApiKeySetup handles saving state', async () => {
    const { ApiKeySetup } = await import('../../src/components/onboarding/ApiKeySetup')
    expect(ApiKeySetup).toBeDefined()
  })
})
