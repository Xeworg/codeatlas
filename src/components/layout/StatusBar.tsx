import { Files, Link2, Clock, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react'
import type { ScanResult, ScanStatus } from '../../lib/types'

interface StatusBarProps {
  scanResult: ScanResult | null
  scanDuration: number | null
  status: ScanStatus
}

const syncLabels: Record<ScanStatus, string> = {
  idle: 'Sin proyecto',
  scanning: 'Escaneando...',
  building_graph: 'Construyendo...',
  ready: 'Sincronizado',
  error: 'Error',
}

const syncColors: Record<ScanStatus, string> = {
  idle: 'text-text-muted',
  scanning: 'text-yellow-400',
  building_graph: 'text-blue-400',
  ready: 'text-green-400',
  error: 'text-red-400',
}

export function StatusBar({ scanResult, scanDuration, status }: StatusBarProps) {
  return (
    <footer className="h-6 bg-surface-elevated border-t border-border-subtle flex items-center px-4 gap-4 text-xs text-text-muted flex-shrink-0">
      <Files size={12} />
      <span>{scanResult ? `${scanResult.filesCount} archivos` : 'Sin datos'}</span>
      <span className="text-text-muted opacity-40">|</span>
      <Link2 size={12} />
      <span>{scanResult ? `${scanResult.importsCount} dependencias` : '—'}</span>
      {scanDuration !== null && (
        <>
          <span className="text-text-muted opacity-40">|</span>
          <Clock size={12} />
          <span>{scanDuration}ms</span>
        </>
      )}

      <div className="flex-1" />

      {/* Right-side sync / status indicator — reflects real ScanStatus */}
      <div className={`flex items-center gap-1.5 ${syncColors[status]}`}>
        {status === 'scanning' || status === 'building_graph' ? (
          <Loader2 size={12} className="animate-spin" />
        ) : status === 'error' ? (
          <AlertCircle size={12} />
        ) : (
          <CheckCircle2 size={12} />
        )}
        <span>{syncLabels[status]}</span>
      </div>
    </footer>
  )
}
