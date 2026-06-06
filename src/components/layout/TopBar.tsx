import { Circle, Sparkles, Settings, Map } from 'lucide-react'
import type { ScanStatus } from '../../lib/types'
import { t } from '../../lib/i18n'

interface TopBarProps {
  projectName: string | null
  status: ScanStatus
  onOpenProject: () => void
}

const statusLabels: Record<ScanStatus, string> = {
  idle: t('status.idle'),
  scanning: t('status.scanning'),
  building_graph: t('status.buildingGraph'),
  ready: t('status.ready'),
  error: t('status.error'),
}

const statusDotColor: Record<ScanStatus, string> = {
  idle: 'text-text-muted',
  scanning: 'text-yellow-400',
  building_graph: 'text-blue-400',
  ready: 'text-green-400',
  error: 'text-red-400',
}

export function TopBar({ projectName, status, onOpenProject }: TopBarProps) {
  return (
    <header className="h-12 bg-surface-elevated border-b border-border-subtle flex items-center px-4 gap-3 flex-shrink-0">
      {/* Branding */}
      <div className="flex items-center gap-2 text-text-primary select-none">
        <Map size={18} strokeWidth={1.75} className="text-accent-primary" />
        <span className="text-sm font-semibold tracking-tight">CodeAtlas</span>
      </div>

      {/* Separator */}
      <span className="w-px h-5 bg-border-subtle" />

      {/* Project name as pseudo-dropdown */}
      <span className="text-sm font-medium text-text-primary truncate max-w-[200px] px-2 py-1 rounded-sm border border-border-subtle hover:border-border-strong hover:bg-surface-hover cursor-default select-none">
        {projectName ?? 'CodeAtlas'}
      </span>

      {/* Status dot + label */}
      <span className="flex items-center gap-1.5 text-xs text-text-secondary">
        <Circle size={8} fill="currentColor" className={statusDotColor[status]} />
        {statusLabels[status]}
      </span>

      <div className="flex-1" />

      {/* Nuevo Chat action */}
      <button
        onClick={() => {}}
        className="flex items-center gap-1.5 px-3 py-1 text-xs text-white bg-accent-secondary border border-accent-secondary rounded-sm hover:bg-accent-secondary/90 transition-colors"
        title={t('topBar.newChatTitle')}
      >
        <Sparkles size={13} strokeWidth={1.75} />
        <span>{t('topBar.newChat')}</span>
      </button>

      {/* Abrir proyecto */}
      <button
        onClick={onOpenProject}
        className="px-3 py-1 text-xs text-text-primary border border-border-subtle rounded-sm hover:border-border-strong hover:bg-surface-hover transition-colors"
      >
        {t('topBar.openProject')}
      </button>

      {/* Settings */}
      <button
        onClick={() => {}}
        className="p-1.5 text-text-secondary hover:text-text-primary hover:bg-surface-hover rounded-sm transition-colors"
        title={t('common.settings')}
      >
        <Settings size={16} strokeWidth={1.75} />
      </button>
    </header>
  )
}
