// analyticsStore.test.ts — v3 PR2 wiring smoke tests
// Strict TDD: RED first, then implement

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { act } from '@testing-library/react'
import { useAnalyticsStore } from '../../stores/analyticsStore'

// ── Mock feature flag ──────────────────────────────────────────────────────
vi.mock('../../stores/featureFlags', () => ({
  V3_H1_ENABLED: true,
}))

// ── Tests ───────────────────────────────────────────────────────────────────

describe('analyticsStore — PR2 wiring smoke tests', () => {
  beforeEach(() => {
    // Reset store to defaults before each test
    const store = useAnalyticsStore.getState()
    act(() => {
      store.resetAnalytics()
    })
  })

  describe('T2.5 — Feature flag v3_h1', () => {
    it('T2.5.1: analyticsStore has v3_h1 feature flag export', async () => {
      const flags = await import('../../stores/featureFlags')
      expect(flags).toHaveProperty('V3_H1_ENABLED')
    })

    it('T2.5.2: V3_H1_ENABLED is boolean (true in dev)', async () => {
      const { V3_H1_ENABLED } = await import('../../stores/featureFlags')
      expect(typeof V3_H1_ENABLED).toBe('boolean')
    })

    it('T2.5.3: store can be reset when flag changes', () => {
      const store = useAnalyticsStore.getState()
      act(() => {
        store.setView('flow-beta')
      })
      expect(useAnalyticsStore.getState().activeView).toBe('flow-beta')
      act(() => {
        store.resetAnalytics()
      })
      expect(useAnalyticsStore.getState().activeView).toBe('architecture')
    })
  })

  describe('T2.1 — AnalyticsViewSelector integration', () => {
    it('T2.1.1: store exposes setView action', () => {
      const store = useAnalyticsStore.getState()
      expect(typeof store.setView).toBe('function')
    })

    it('T2.1.2: setView changes activeView to architecture', () => {
      act(() => {
        useAnalyticsStore.getState().setView('architecture')
      })
      expect(useAnalyticsStore.getState().activeView).toBe('architecture')
    })

    it('T2.1.3: setView changes activeView to dependencies', () => {
      act(() => {
        useAnalyticsStore.getState().setView('dependencies')
      })
      expect(useAnalyticsStore.getState().activeView).toBe('dependencies')
    })

    it('T2.1.4: setView changes activeView to flow-beta', () => {
      act(() => {
        useAnalyticsStore.getState().setView('flow-beta')
      })
      expect(useAnalyticsStore.getState().activeView).toBe('flow-beta')
    })
  })

  describe('T2.2 — ArchitectureCard data path', () => {
    it('T2.2.1: store does not hold architecture detection directly (fetched per-project)', () => {
      // ArchitectureCard receives detection as prop; no need for store property
      const state = useAnalyticsStore.getState()
      expect(state).not.toHaveProperty('architectureDetection')
    })
  })

  describe('T2.3 — ImpactPanel data path', () => {
    it('T2.3.1: store does not hold impact analysis directly (fetched per-node)', () => {
      // ImpactPanel receives impact as prop; no need for store property
      const state = useAnalyticsStore.getState()
      expect(state).not.toHaveProperty('impactAnalysis')
    })
  })

  describe('T2.4 — InsightsPanel data path', () => {
    it('T2.4.1: store does not hold graph insights directly (fetched per-project)', () => {
      // InsightsPanel receives insights as prop; no need for store property
      const state = useAnalyticsStore.getState()
      expect(state).not.toHaveProperty('graphInsights')
    })
  })

  describe('T2.6 — Reset analytics', () => {
    it('T2.6.1: resetAnalytics returns store to defaults', () => {
      const store = useAnalyticsStore.getState()
      act(() => {
        store.setView('flow-beta')
        store.setFilter('nodeTypeFilter', 'service')
        store.setFilter('couplingThreshold', 0.9)
        store.setFilter('showCycles', true)
        store.setFilter('showHotspots', true)
      })

      act(() => {
        store.resetAnalytics()
      })

      const s = useAnalyticsStore.getState()
      expect(s.activeView).toBe('architecture')
      expect(s.nodeTypeFilter).toBeNull()
      expect(s.couplingThreshold).toBe(0.5)
      expect(s.showCycles).toBe(false)
      expect(s.showHotspots).toBe(false)
    })
  })
})
