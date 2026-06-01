import type { ReactNode } from 'react'
import type { ScanResult, ScanStatus } from '../../lib/types'
import { TopBar } from './TopBar'
import { StatusBar } from './StatusBar'

interface AppShellProps {
  projectName: string | null
  status: ScanStatus
  scanResult: ScanResult | null
  scanDuration: number | null
  onOpenProject: () => void
  sidebar: ReactNode
  children: ReactNode
  rightPanel?: ReactNode
}

export function AppShell({
  projectName,
  status,
  scanResult,
  scanDuration,
  onOpenProject,
  sidebar,
  children,
  rightPanel,
}: AppShellProps) {
  return (
    <div className="flex flex-col h-screen bg-gray-900 text-gray-100">
      <TopBar projectName={projectName} status={status} onOpenProject={onOpenProject} />
      <div className="flex flex-1 overflow-hidden">
        {sidebar}
        <main className="flex-1 overflow-hidden relative">{children}</main>
        {rightPanel && (
          <aside className="w-80 bg-gray-850 border-l border-gray-700 flex flex-col overflow-hidden">
            {rightPanel}
          </aside>
        )}
      </div>
      <StatusBar scanResult={scanResult} scanDuration={scanDuration} />
    </div>
  )
}
