import { Circle } from 'lucide-react'
import type { ScanStatus } from '../../lib/types'

interface TopBarProps {
  projectName: string | null
  status: ScanStatus
  onOpenProject: () => void
}

const statusLabels: Record<ScanStatus, string> = {
  idle: 'Sin proyecto',
  scanning: 'Escaneando...',
  building_graph: 'Construyendo grafo...',
  ready: 'Listo',
  error: 'Error',
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

      <button
        onClick={onOpenProject}
        className="px-3 py-1 text-xs text-text-primary border border-border-subtle rounded-sm hover:border-border-strong hover:bg-surface-hover transition-colors"
      >
        Abrir proyecto
      </button>
    </header>
  )
}
