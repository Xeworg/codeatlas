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
    <div className="flex flex-col h-screen bg-surface-base text-text-primary">
      <TopBar projectName={projectName} status={status} onOpenProject={onOpenProject} />
      <div className="flex flex-1 overflow-hidden">
        {sidebar}
        <main className="flex-1 overflow-hidden relative">{children}</main>
        {rightPanel && (
          <aside className="w-96 bg-surface-elevated border-l border-border-subtle flex flex-col overflow-hidden">
            {rightPanel}
          </aside>
        )}
      </div>
      <StatusBar scanResult={scanResult} scanDuration={scanDuration} status={status} />
    </div>
  )
}
