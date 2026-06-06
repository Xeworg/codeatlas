// AnalyticsViewSelector — toolbar for switching between analytical views
// Part of PR5 (migrated to i18n in PR6)

import { Map, GitBranch, Workflow, type LucideIcon } from 'lucide-react'
import { t } from '../../lib/i18n'
import { useAnalyticsStore, useActiveView, type AnalyticalView } from '../../stores/analyticsStore'

interface ViewOption {
  id: AnalyticalView
  label: string
  beta?: boolean
}

const VIEW_ICONS: Record<AnalyticalView, LucideIcon> = {
  architecture: Map,
  dependencies: GitBranch,
  'flow-beta': Workflow,
}

const VIEWS: ViewOption[] = [
  { id: 'architecture', label: t('views.architecture') },
  { id: 'dependencies', label: t('views.dependencies') },
  { id: 'flow-beta', label: t('views.flowBeta'), beta: true },
]

export function AnalyticsViewSelector() {
  const activeView = useActiveView()
  const setView = useAnalyticsStore((s) => s.setView)

  return (
    <div className="flex items-center gap-1 bg-surface-elevated border-b border-border-subtle px-3 py-2 flex-shrink-0">
      <span className="text-xs text-text-muted mr-2 font-semibold uppercase tracking-wide">
        {t('common.view')}
      </span>
      {VIEWS.map((view) => {
        const Icon = VIEW_ICONS[view.id]
        return (
          <button
            key={view.id}
            role="button"
            aria-pressed={activeView === view.id}
            onClick={() => setView(view.id)}
            className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs font-semibold transition-all ${
              activeView === view.id
                ? 'bg-accent-primary text-white shadow'
                : 'text-text-muted hover:text-text-secondary hover:bg-surface-hover'
            }`}
          >
            <Icon size={13} />
            {view.label}
            {view.beta && (
              <span className="ml-1 text-[9px] bg-amber-600 text-white px-1 rounded align-middle">
                beta
              </span>
            )}
          </button>
        )
      })}
    </div>
  )
}
