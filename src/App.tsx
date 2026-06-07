// App — main entry, wires all panels together
// Layout pass: assistant/AI moved to right panel; main stack simplified
// Part of PR-8 (Frontend services/hooks): orchestration moved to hooks

import { useState, useCallback, useEffect, useRef } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { t } from './lib/i18n'
import { FileText, Zap, FolderOpen, HelpCircle } from 'lucide-react'
import { AppShell } from './components/layout/AppShell'
import { Sidebar } from './components/layout/Sidebar'
import { EmptyState } from './components/common/EmptyState'
import { ErrorState } from './components/common/ErrorState'
import { Spinner } from './components/common/Spinner'
import { TabSwitcher } from './components/common/TabSwitcher'
import { GraphView } from './components/graph/GraphView'
import { SearchOverlay } from './components/graph/SearchOverlay'
import { DetailPanel } from './components/panel/DetailPanel'
import { AIExplanation } from './components/panel/AIExplanation'
import { ChatPanel } from './components/chat/ChatPanel'
import { AnalyticsViewSelector, ArchitectureCard, ImpactPanel } from './components/analytics'
import { useProjectStore, useScanStatus } from './stores/projectStore'
import { useGraphStore, useSelectedNodeId } from './stores/graphStore'
import { useProject } from './hooks/useProject'
import { useArchitecture } from './hooks/useArchitecture'
import { V3_H1_ENABLED } from './stores/featureFlags'

type DetailTab = 'details' | 'impact'

interface DetailTabDef {
  id: DetailTab
  label: string
  icon: React.ReactNode
}

const V2_DETAIL_TABS: DetailTabDef[] = [
  { id: 'details', label: t('tabs.summary'), icon: <FileText size={14} strokeWidth={1.75} /> },
]

function App() {
  const status = useScanStatus()
  const selectedNodeId = useSelectedNodeId()
  const projectId = useProjectStore((s) => s.projectId)
  const { scanResult, projectName, error } = useProjectStore()
  const { selectNode, graphData } = useGraphStore()
  const { openProject } = useProject()
  const { architectureDetection, impactAnalysis } = useArchitecture()

  const [scanStartTime, setScanStartTime] = useState<number | null>(null)
  const [detailTab, setDetailTab] = useState<DetailTab>('details')

  // Auto-select Impact tab when impact analysis arrives
  const prevImpactNode = useRef<string | null>(null)
  const autoSelectImpact = useCallback(() => {
    if (impactAnalysis && selectedNodeId && V3_H1_ENABLED) {
      if (selectedNodeId !== prevImpactNode.current) {
        prevImpactNode.current = selectedNodeId
        setDetailTab('impact')
      }
    }
  }, [impactAnalysis, selectedNodeId])
  useEffect(() => {
    autoSelectImpact()
  }, [autoSelectImpact])

  const handleOpenProject = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false })
    if (!selected) return
    setScanStartTime(Date.now())
    await openProject(selected as string)
  }, [openProject])

  const handleSelectFile = useCallback(
    (fileId: string) => {
      selectNode(fileId)
    },
    [selectNode]
  )

  // ── Detail panel content ──────────────────────────────────────────────────
  const renderDetailContent = () => {
    if (V3_H1_ENABLED && detailTab === 'impact') {
      if (!impactAnalysis) {
        return (
          <div className="flex items-center justify-center h-full">
            <Spinner />
          </div>
        )
      }
      return <ImpactPanel impact={impactAnalysis} />
    }

    return <DetailPanel />
  }

  const detailTabs = V3_H1_ENABLED
    ? [
        ...V2_DETAIL_TABS,
        {
          id: 'impact' as DetailTab,
          label: t('tabs.impact'),
          icon: <Zap size={14} strokeWidth={1.75} />,
        },
      ]
    : V2_DETAIL_TABS

  // ── Right panel: persistent assistant + AI explanation + architecture card
  // ChatPanel gets its own scroll viewport so long history does not push other cards off-screen.
  const rightPanel =
    status === 'ready' ? (
      <div className="flex flex-col h-full overflow-y-auto">
        <div className="h-[55vh] flex flex-col overflow-hidden flex-shrink-0">
          <ChatPanel
            projectId={projectId}
            contextNodeIds={selectedNodeId ? [selectedNodeId] : []}
          />
        </div>
        {selectedNodeId && (
          <AIExplanation
            nodeId={selectedNodeId}
            projectId={projectId}
            nodeLabel={graphData?.nodes.find((n) => n.id === selectedNodeId)?.label}
          />
        )}
        {V3_H1_ENABLED && architectureDetection && (
          <ArchitectureCard detection={architectureDetection} />
        )}
      </div>
    ) : undefined

  const mainContent = () => {
    if (error)
      return (
        <ErrorState message={error} onRetry={handleOpenProject} actionLabel={t('common.retry')} />
      )

    if (status === 'idle') {
      return (
        <EmptyState
          icon={<FolderOpen size={24} strokeWidth={1.5} />}
          title={t('app.emptyProjectTitle')}
          description={t('app.emptyProjectDescription')}
          action={{ label: t('app.openProjectAction'), onClick: handleOpenProject }}
        />
      )
    }

    if (status === 'scanning' || status === 'building_graph') {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <Spinner size="lg" />
            <p className="mt-3 text-sm text-text-muted">
              {status === 'scanning' ? t('app.scanningFiles') : t('app.buildingGraph')}
            </p>
          </div>
        </div>
      )
    }

    if (status === 'ready') {
      return (
        <div className="flex flex-col h-full overflow-hidden">
          {/* Analytics tabs toolbar */}
          {V3_H1_ENABLED && <AnalyticsViewSelector />}

          {/* Main graph area */}
          <div className="flex-1 relative overflow-hidden">
            <GraphView />
            <SearchOverlay />
          </div>

          {/* Detail panel */}
          {selectedNodeId && (
            <div className="h-72 border-t border-border-subtle overflow-hidden flex-shrink-0 flex flex-col bg-surface-base">
              <TabSwitcher
                tabs={detailTabs}
                activeTab={detailTab}
                onChange={(id) => setDetailTab(id as DetailTab)}
              />
              <div className="flex-1 overflow-y-auto">{renderDetailContent()}</div>
            </div>
          )}
        </div>
      )
    }

    return (
      <EmptyState icon={<HelpCircle size={24} strokeWidth={1.5} />} title={t('app.unknownState')} />
    )
  }

  const scanDuration = scanStartTime ? Date.now() - scanStartTime : null

  return (
    <AppShell
      projectName={projectName}
      status={status}
      scanResult={scanResult}
      scanDuration={scanDuration}
      onOpenProject={handleOpenProject}
      sidebar={
        <Sidebar
          scanResult={scanResult}
          selectedFileId={selectedNodeId}
          onSelectFile={handleSelectFile}
          onSearch={(_q) => {}}
        />
      }
      rightPanel={rightPanel}
    >
      {mainContent()}
    </AppShell>
  )
}

export default App
