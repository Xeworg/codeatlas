// analytics.test.ts — RED tests for PR5 analytics UX
// Tests store, components, and wiring for v2 analytical views.

import { describe, it, expect, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import React from 'react'
import { useAnalyticsStore } from '../../src/stores/analyticsStore'
import type {
  ArchitectureDetectionResult,
  ImpactAnalysisResult,
  GraphInsights,
} from '../../src/lib/types'

// ─── T5.1: Store tests ────────────────────────────────────────────────

describe('T5.1 — useAnalyticsStore', () => {
  beforeEach(() => {
    useAnalyticsStore.getState().resetAnalytics()
  })

  it('has default view architecture', () => {
    expect(useAnalyticsStore.getState().activeView).toBe('architecture')
  })

  it('setView changes activeView', () => {
    useAnalyticsStore.getState().setView('dependencies')
    expect(useAnalyticsStore.getState().activeView).toBe('dependencies')
  })

  it('setView supports flow-beta', () => {
    useAnalyticsStore.getState().setView('flow-beta')
    expect(useAnalyticsStore.getState().activeView).toBe('flow-beta')
  })

  it('setFilter sets a single filter value', () => {
    useAnalyticsStore.getState().setFilter('nodeTypeFilter', 'Service')
    expect(useAnalyticsStore.getState().nodeTypeFilter).toBe('Service')
  })

  it('setFilter updates couplingThreshold', () => {
    useAnalyticsStore.getState().setFilter('couplingThreshold', 0.7)
    expect(useAnalyticsStore.getState().couplingThreshold).toBe(0.7)
  })

  it('setFilter toggles showCycles', () => {
    useAnalyticsStore.getState().setFilter('showCycles', true)
    expect(useAnalyticsStore.getState().showCycles).toBe(true)
  })

  it('setFilter toggles showHotspots', () => {
    useAnalyticsStore.getState().setFilter('showHotspots', false)
    expect(useAnalyticsStore.getState().showHotspots).toBe(false)
  })

  it('resetFilters restores all defaults', () => {
    // setView is NOT reset by resetFilters (only filters)
    useAnalyticsStore.getState().setView('dependencies')
    useAnalyticsStore.getState().setFilter('nodeTypeFilter', 'Component')
    useAnalyticsStore.getState().setFilter('couplingThreshold', 0.9)
    useAnalyticsStore.getState().setFilter('showCycles', true)
    useAnalyticsStore.getState().resetFilters()
    // activeView unchanged by resetFilters (only filters reset)
    expect(useAnalyticsStore.getState().activeView).toBe('dependencies')
    expect(useAnalyticsStore.getState().nodeTypeFilter).toBe(null)
    expect(useAnalyticsStore.getState().couplingThreshold).toBe(0.5)
    expect(useAnalyticsStore.getState().showCycles).toBe(false)
    expect(useAnalyticsStore.getState().showHotspots).toBe(false)
  })

  it('filters persist across view changes (session)', () => {
    useAnalyticsStore.getState().setFilter('nodeTypeFilter', 'Model')
    useAnalyticsStore.getState().setFilter('showCycles', true)
    useAnalyticsStore.getState().setView('dependencies')
    expect(useAnalyticsStore.getState().nodeTypeFilter).toBe('Model')
    expect(useAnalyticsStore.getState().showCycles).toBe(true)
  })
})

// ─── T5.2: ArchitectureCard component tests ────────────────────────────

describe('T5.2 — ArchitectureCard', () => {
  it('renders detected pattern with confidence badge', async () => {
    const { ArchitectureCard } = await import('../../src/components/analytics/ArchitectureCard')
    const result: ArchitectureDetectionResult = {
      version: '2.0',
      pattern: 'clean',
      confidence: 0.85,
      evidence: {
        nodes: ['src/domain/User.ts', 'src/application/UseCase.ts'],
        edges: [],
        reasons: ['domain folder detected', 'application folder detected'],
      },
      generatedAt: new Date().toISOString(),
    }
    render(<ArchitectureCard detection={result} />)
    expect(screen.getByText(/clean/i)).toBeInTheDocument()
    expect(screen.getByText(/85%/)).toBeInTheDocument()
  })

  it('renders unknown pattern with appropriate message', async () => {
    const { ArchitectureCard } = await import('../../src/components/analytics/ArchitectureCard')
    const result: ArchitectureDetectionResult = {
      version: '2.0',
      pattern: 'unknown',
      confidence: 0,
      evidence: null,
      generatedAt: new Date().toISOString(),
    }
    render(<ArchitectureCard detection={result} />)
    expect(screen.getByText(/sin arquitectura detectada/i)).toBeInTheDocument()
  })

  it('shows evidence when expanded', async () => {
    const { ArchitectureCard } = await import('../../src/components/analytics/ArchitectureCard')
    const result: ArchitectureDetectionResult = {
      version: '2.0',
      pattern: 'mvc',
      confidence: 0.6,
      evidence: {
        nodes: ['controllers/UserController.ts'],
        edges: [],
        reasons: ['controllers folder detected'],
      },
      generatedAt: new Date().toISOString(),
    }
    render(<ArchitectureCard detection={result} />)
    const toggle = screen.getByRole('button', { name: /evidencia/i })
    fireEvent.click(toggle)
    expect(screen.getByText(/controllers\/UserController/i)).toBeInTheDocument()
  })
})

// ─── T5.3: ImpactPanel component tests ─────────────────────────────────

describe('T5.3 — ImpactPanel', () => {
  it('renders impact result with affected nodes list', async () => {
    const { ImpactPanel } = await import('../../src/components/analytics/ImpactPanel')
    const impact: ImpactAnalysisResult = {
      version: '2.0',
      changedNodeId: 'file-a',
      affectedNodes: ['file-b', 'file-c'],
      impactScore: 0.65,
      explanation: 'file-a imports file-b, file-b imports file-c',
    }
    const { container } = render(<ImpactPanel impact={impact} />)
    expect(container.textContent).toContain('file-b')
    expect(container.textContent).toContain('file-c')
    expect(container.textContent).toContain('65')
  })

  it('renders empty state when no affected nodes', async () => {
    const { ImpactPanel } = await import('../../src/components/analytics/ImpactPanel')
    const impact: ImpactAnalysisResult = {
      version: '2.0',
      changedNodeId: 'file-a',
      affectedNodes: [],
      impactScore: 0,
      explanation: 'No dependencies found',
    }
    const { container } = render(<ImpactPanel impact={impact} />)
    expect(container.textContent).toContain('Sin impacto')
  })
})

// ─── T5.4: InsightsPanel component tests ─────────────────────────────────

describe('T5.4 — InsightsPanel', () => {
  it('renders cycles tab with cycle data', async () => {
    const { InsightsPanel } = await import('../../src/components/analytics/InsightsPanel')
    const insights: GraphInsights = {
      version: '2.0',
      cycles: [{ nodes: ['A', 'B'], length: 2 }],
      hotspots: [{ nodeId: 'C', couplingScore: 0.9, reason: 'high in-degree' }],
      avgCoupling: 0.45,
      density: 0.12,
      status: 'ok',
    }
    render(<InsightsPanel insights={insights} />)
    expect(screen.getByText(/Ciclo #1/)).toBeInTheDocument()
    expect(screen.getByText(/(2 nodos)/)).toBeInTheDocument()
  })

  it('renders metrics tab with avgCoupling and density', async () => {
    const { InsightsPanel } = await import('../../src/components/analytics/InsightsPanel')
    const insights: GraphInsights = {
      version: '2.0',
      cycles: [],
      hotspots: [],
      avgCoupling: 0.45,
      density: 0.12,
      status: 'ok',
    }
    render(<InsightsPanel insights={insights} />)
    const metricsTab = screen.getByRole('tab', { name: /métricas/i })
    fireEvent.click(metricsTab)
    expect(screen.getByText('0.450')).toBeInTheDocument()
    expect(screen.getByText('0.120')).toBeInTheDocument()
  })

  it('renders empty state for no cycles', async () => {
    const { InsightsPanel } = await import('../../src/components/analytics/InsightsPanel')
    const insights: GraphInsights = {
      version: '2.0',
      cycles: [],
      hotspots: [],
      avgCoupling: 0,
      density: 0,
      status: 'ok',
    }
    render(<InsightsPanel insights={insights} />)
    expect(screen.getByText(/sin ciclos/i)).toBeInTheDocument()
  })
})

// ─── T5.5: AnalyticsViewSelector tests ──────────────────────────────────

describe('T5.5 — AnalyticsViewSelector', () => {
  beforeEach(() => {
    useAnalyticsStore.getState().resetAnalytics()
  })

  it('renders three view buttons', async () => {
    const { AnalyticsViewSelector } =
      await import('../../src/components/analytics/AnalyticsViewSelector')
    render(<AnalyticsViewSelector />)
    expect(screen.getByRole('button', { name: /arquitectura/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /dependencias/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /flujo/i })).toBeInTheDocument()
  })

  it('sets activeView on click', async () => {
    const { AnalyticsViewSelector } =
      await import('../../src/components/analytics/AnalyticsViewSelector')
    render(<AnalyticsViewSelector />)
    fireEvent.click(screen.getByRole('button', { name: /dependencias/i }))
    expect(useAnalyticsStore.getState().activeView).toBe('dependencies')
  })
})
