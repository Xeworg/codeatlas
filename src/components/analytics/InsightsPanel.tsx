// InsightsPanel — shows graph insights: cycles, hotspots, and metrics
// Part of PR5 (migrated to i18n in PR6)

import { useState } from 'react'
import { t } from '../../lib/i18n'
import type { GraphInsights } from '../../lib/types'

interface InsightsPanelProps {
  insights: GraphInsights
}

type InsightsTab = 'cycles' | 'hotspots' | 'metrics'

const TABS: { id: InsightsTab; label: string }[] = [
  { id: 'cycles', label: t('insights.tabs.cycles') },
  { id: 'hotspots', label: t('insights.tabs.hotspots') },
  { id: 'metrics', label: t('insights.tabs.metrics') },
]

export function InsightsPanel({ insights }: InsightsPanelProps) {
  const [activeTab, setActiveTab] = useState<InsightsTab>('cycles')

  const { cycles, hotspots, avgCoupling, density, status } = insights

  const renderTabContent = () => {
    switch (activeTab) {
      case 'cycles':
        if (cycles.length === 0) {
          return (
            <p className="text-sm text-text-muted italic p-4 text-center">
              {t('insights.noCycles')}
            </p>
          )
        }
        return (
          <ul className="space-y-2 p-3 max-h-52 overflow-y-auto">
            {cycles.map((cycle, i) => (
              <li key={i} className="bg-red-900/30 border border-red-800 rounded p-2 text-xs">
                <span className="text-red-400 font-semibold">
                  {t('insights.cycleNumber', { number: String(i + 1) })}
                </span>
                <span className="text-text-muted ml-2">
                  {t('insights.cycleNodes', { count: String(cycle.length) })}
                </span>
                <div className="mt-1 space-y-0.5">
                  {cycle.nodes.map((node, j) => (
                    <span key={j} className="text-text-secondary font-mono">
                      {node}
                      {j < cycle.nodes.length - 1 ? ' → ' : ''}
                    </span>
                  ))}
                </div>
              </li>
            ))}
          </ul>
        )

      case 'hotspots':
        if (hotspots.length === 0) {
          return (
            <p className="text-sm text-text-muted italic p-4 text-center">
              {t('insights.noHotspots')}
            </p>
          )
        }
        return (
          <ul className="space-y-2 p-3 max-h-52 overflow-y-auto">
            {hotspots.map((spot) => (
              <li
                key={spot.nodeId}
                className="bg-amber-900/30 border border-amber-800 rounded p-2 flex items-start justify-between gap-2"
              >
                <div className="min-w-0">
                  <p className="text-xs font-mono text-text-primary truncate">{spot.nodeId}</p>
                  <p className="text-xs text-text-muted mt-0.5">{spot.reason}</p>
                </div>
                <span className="text-amber-400 font-bold text-sm flex-shrink-0">
                  {Math.round(spot.couplingScore * 100)}%
                </span>
              </li>
            ))}
          </ul>
        )

      case 'metrics':
        return (
          <div className="p-4 space-y-3">
            <div className="flex justify-between items-center py-2 border-b border-border-subtle">
              <span className="text-xs text-text-muted">{t('insights.avgCoupling')}</span>
              <span className="text-sm font-mono font-semibold text-text-secondary">
                {avgCoupling != null ? avgCoupling.toFixed(3) : '—'}
              </span>
            </div>
            <div className="flex justify-between items-center py-2 border-b border-border-subtle">
              <span className="text-xs text-text-muted">{t('insights.graphDensity')}</span>
              <span className="text-sm font-mono font-semibold text-text-secondary">
                {density != null ? density.toFixed(3) : '—'}
              </span>
            </div>
            <div className="flex justify-between items-center py-2">
              <span className="text-xs text-text-muted">{t('insights.totalCycles')}</span>
              <span className="text-sm font-mono font-semibold text-red-400">{cycles.length}</span>
            </div>
            <div className="flex justify-between items-center py-2">
              <span className="text-xs text-text-muted">{t('insights.totalHotspots')}</span>
              <span className="text-sm font-mono font-semibold text-amber-400">
                {hotspots.length}
              </span>
            </div>
            <div className="flex justify-between items-center py-2">
              <span className="text-xs text-text-muted">{t('insights.status')}</span>
              <span
                className={`text-xs px-2 py-0.5 rounded font-semibold ${
                  status === 'ok'
                    ? 'bg-emerald-900/50 text-emerald-400'
                    : 'bg-red-900/50 text-red-400'
                }`}
              >
                {status === 'ok'
                  ? t('insights.statusOk')
                  : status === 'timeout'
                    ? t('insights.statusTimeout')
                    : t('insights.statusError')}
              </span>
            </div>
          </div>
        )
    }
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Tabs */}
      <div className="flex border-b border-border-subtle flex-shrink-0" role="tablist">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={activeTab === tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex-1 py-2 text-xs font-semibold transition-colors ${
              activeTab === tab.id
                ? 'text-text-primary border-b-2 border-border-accent bg-surface-elevated'
                : 'text-text-muted hover:text-text-secondary hover:bg-surface-hover'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">{renderTabContent()}</div>
    </div>
  )
}
