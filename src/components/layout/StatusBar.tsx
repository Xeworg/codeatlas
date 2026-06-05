import type { ScanResult } from '../../lib/types'

interface StatusBarProps {
  scanResult: ScanResult | null
  scanDuration: number | null
}

export function StatusBar({ scanResult, scanDuration }: StatusBarProps) {
  return (
    <footer className="h-6 bg-gray-900 border-t border-gray-700 flex items-center px-4 gap-4 text-xs text-gray-400 flex-shrink-0">
      <span>{scanResult ? `${scanResult.filesCount} archivos` : 'Sin datos'}</span>
      <span className="text-gray-600">|</span>
      <span>{scanResult ? `${scanResult.importsCount} dependencias` : '—'}</span>
      {scanDuration !== null && (
        <>
          <span className="text-gray-600">|</span>
          <span>{scanDuration}ms</span>
        </>
      )}
    </footer>
  )
}
