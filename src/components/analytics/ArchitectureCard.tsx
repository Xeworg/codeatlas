// ArchitectureCard — shows detected architecture pattern with confidence and evidence
// Part of PR5 (migrated to i18n in PR6)

import { useState } from 'react'
import { t } from '../../lib/i18n'
import type { ArchitectureDetectionResult } from '../../lib/types'

interface ArchitectureCardProps {
  detection: ArchitectureDetectionResult
}

const PATTERN_LABELS: Record<string, string> = {
  mvc: t('architecture.pattern.mvc'),
  layered: t('architecture.pattern.layered'),
  clean: t('architecture.pattern.clean'),
  hexagonal: t('architecture.pattern.hexagonal'),
  unknown: t('architecture.pattern.unknown'),
}

const CONFIDENCE_COLOR = (confidence: number): string => {
  if (confidence >= 0.7) return 'text-emerald-400'
  if (confidence >= 0.4) return 'text-amber-400'
  return 'text-slate-400'
}

export function ArchitectureCard({ detection }: ArchitectureCardProps) {
  const [showEvidence, setShowEvidence] = useState(false)

  const { pattern, confidence, evidence } = detection
  const label = PATTERN_LABELS[pattern] ?? pattern
  const isUnknown = pattern === 'unknown'

  return (
    <div className="bg-surface-elevated border border-border-subtle rounded-lg p-4 space-y-3">
      {/* Header */}
      <div className="flex items-start justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-text-secondary uppercase tracking-wide">
            {t('architecture.title')}
          </h3>
          <p className="text-lg font-bold text-text-primary mt-0.5">
            {isUnknown ? <span className="text-text-muted italic">{label}</span> : label}
          </p>
        </div>
        {!isUnknown && (
          <span
            className={`text-2xl font-mono font-bold ${CONFIDENCE_COLOR(confidence)}`}
            title={t('architecture.confidence', { value: String(Math.round(confidence * 100)) })}
          >
            {Math.round(confidence * 100)}%
          </span>
        )}
      </div>

      {/* Confidence bar */}
      {!isUnknown && (
        <div className="w-full bg-surface-inset rounded-full h-1.5">
          <div
            className={`h-1.5 rounded-full transition-all ${
              confidence >= 0.7
                ? 'bg-emerald-500'
                : confidence >= 0.4
                  ? 'bg-amber-500'
                  : 'bg-slate-500'
            }`}
            style={{ width: `${Math.round(confidence * 100)}%` }}
          />
        </div>
      )}

      {/* Evidence toggle */}
      {evidence && evidence.nodes.length > 0 && (
        <div className="space-y-2">
          <button
            onClick={() => setShowEvidence((v) => !v)}
            className="text-xs text-text-muted hover:text-text-secondary underline"
          >
            {showEvidence ? t('architecture.hideEvidence') : t('architecture.showEvidence')} (
            {t('architecture.filesCount', { count: String(evidence.nodes.length) })},{' '}
            {t('architecture.reasonsCount', { count: String(evidence.reasons.length) })})
          </button>

          {showEvidence && (
            <div className="bg-surface-base rounded p-2 space-y-2 text-xs max-h-48 overflow-y-auto">
              {evidence.reasons.length > 0 && (
                <div>
                  <p className="text-text-muted font-semibold mb-1">{t('architecture.reasons')}</p>
                  <ul className="space-y-0.5">
                    {evidence.reasons.map((r, i) => (
                      <li key={i} className="text-text-secondary">
                        • {r}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              {evidence.nodes.length > 0 && (
                <div>
                  <p className="text-text-muted font-semibold mb-1 mt-2">
                    {t('architecture.files')}
                  </p>
                  <ul className="space-y-0.5">
                    {evidence.nodes.map((n, i) => (
                      <li key={i} className="text-text-muted font-mono break-all">
                        {n}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
