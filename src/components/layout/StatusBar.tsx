import { Files, Link2, Clock } from 'lucide-react'
import type { ScanResult } from '../../lib/types'

interface StatusBarProps {
  scanResult: ScanResult | null
  scanDuration: number | null
}

export function StatusBar({ scanResult, scanDuration }: StatusBarProps) {
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
    </footer>
  )
}
