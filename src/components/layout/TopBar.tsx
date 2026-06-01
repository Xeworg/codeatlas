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

const statusColors: Record<ScanStatus, string> = {
  idle: 'text-gray-400',
  scanning: 'text-yellow-400',
  building_graph: 'text-blue-400',
  ready: 'text-green-400',
  error: 'text-red-400',
}

export function TopBar({ projectName, status, onOpenProject }: TopBarProps) {
  return (
    <header className="h-12 bg-gray-900 border-b border-gray-700 flex items-center px-4 gap-4 flex-shrink-0">
      <span className="text-sm font-medium text-gray-200 truncate max-w-[200px]">
        {projectName ?? 'CodeAtlas'}
      </span>
      <span className={`text-xs ${statusColors[status]}`}>{statusLabels[status]}</span>
      <div className="flex-1" />
      <button
        onClick={onOpenProject}
        className="px-3 py-1 text-xs bg-blue-600 hover:bg-blue-500 text-white rounded transition-colors"
      >
        Abrir proyecto
      </button>
    </header>
  )
}
