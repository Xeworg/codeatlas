// ImpactPanel — shows impact analysis result for selected node
// Part of PR5 (migrated to i18n in PR6)

import { CornerDownRight } from 'lucide-react'
import { t } from '../../lib/i18n'
import type { ImpactAnalysisResult } from '../../lib/types'

interface ImpactPanelProps {
  impact: ImpactAnalysisResult
}

const SCORE_COLOR = (score: number): string => {
  if (score >= 0.7) return 'text-red-400'
  if (score >= 0.4) return 'text-amber-400'
  return 'text-slate-400'
}

const SCORE_BG = (score: number): string => {
  if (score >= 0.7) return 'bg-red-900/40 border-red-800'
  if (score >= 0.4) return 'bg-amber-900/40 border-amber-800'
  return 'bg-surface-elevated border-border-subtle'
}

export function ImpactPanel({ impact }: ImpactPanelProps) {
  const { affectedNodes, impactScore, explanation } = impact

  if (affectedNodes.length === 0) {
    return (
      <div className="p-4 space-y-3">
        <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wide">
          {t('impact.title')}
        </h3>
        <div className="bg-surface-elevated border border-border-subtle rounded-lg p-4 text-center">
          <p className="text-sm text-text-muted italic">{t('impact.noImpact')}</p>
        </div>
        {explanation && <p className="text-xs text-text-muted italic">{explanation}</p>}
      </div>
    )
  }

  return (
    <div className="p-4 space-y-3">
      <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wide">
        {t('impact.title')}
      </h3>

      {/* Score badge */}
      <div
        className={`rounded-lg p-3 border ${SCORE_BG(impactScore)} flex items-center justify-between`}
      >
        <span className="text-xs text-text-muted">{t('impact.estimatedImpact')}</span>
        <span className={`text-xl font-bold font-mono ${SCORE_COLOR(impactScore)}`}>
          {Math.round(impactScore * 100)}%
        </span>
      </div>

      {/* Explanation */}
      {explanation && <p className="text-xs text-text-muted leading-relaxed">{explanation}</p>}

      {/* Affected nodes */}
      <div>
        <p className="text-xs text-text-muted font-semibold mb-2">
          {t('impact.affectedFiles', { count: String(affectedNodes.length) })}:
        </p>
        <ul className="space-y-1 max-h-40 overflow-y-auto">
          {affectedNodes.map((nodeId) => (
            <li
              key={nodeId}
              className="text-xs text-text-secondary font-mono flex items-center gap-1.5 bg-surface-elevated rounded px-2 py-1"
            >
              <CornerDownRight size={12} className="text-text-muted flex-shrink-0" />
              <span className="truncate">{nodeId}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}
