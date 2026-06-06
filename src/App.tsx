// App — main entry, wires all panels together
// Layout pass: assistant/AI moved to right panel; main stack simplified

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
import {
  scanProject,
  openProjectByPath,
  getGraph,
  getErrorMessage,
  getArchitectureDetection,
  getImpactAnalysis,
} from './lib/tauri-api'
import { buildLayout } from './lib/graph-layout'
import { V3_H1_ENABLED } from './stores/featureFlags'
import type { ArchitectureDetectionResult, ImpactAnalysisResult, ScanResult } from './lib/types'

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
  const { scanResult, projectName, error, setProject, setScanResult, setStatus, setError } =
    useProjectStore()
  const { selectNode, setGraphData, setLoading, graphData } = useGraphStore()

  const [scanStartTime, setScanStartTime] = useState<number | null>(null)
  const [detailTab, setDetailTab] = useState<DetailTab>('details')

  // ── Analytics state ────────────────────────────────────────────────────
  const [architectureDetection, setArchitectureDetection] =
    useState<ArchitectureDetectionResult | null>(null)
  const [impactAnalysis, setImpactAnalysis] = useState<ImpactAnalysisResult | null>(null)

  // Track previous projectId to re-fetch analytics on project change
  const prevProjectId = useRef<string | null>(null)

  // ── Fetch architecture detection when project is ready ─────────────────
  useEffect(() => {
    if (!V3_H1_ENABLED) return
    if (status !== 'ready' || !projectId) return
    if (projectId === prevProjectId.current) return
    prevProjectId.current = projectId

    setArchitectureDetection(null)
    getArchitectureDetection(projectId)
      .then(setArchitectureDetection)
      .catch(() => setArchitectureDetection(null))
  }, [status, projectId])

  // ── Fetch impact analysis when node is selected ────────────────────────
  useEffect(() => {
    if (!V3_H1_ENABLED) return
    if (status !== 'ready' || !projectId || !selectedNodeId) {
      setImpactAnalysis(null)
      return
    }

    setImpactAnalysis(null)
    getImpactAnalysis(projectId, selectedNodeId)
      .then(setImpactAnalysis)
      .catch(() => setImpactAnalysis(null))
  }, [status, projectId, selectedNodeId])

  // Auto-select Impact tab when impact analysis arrives
  useEffect(() => {
    if (impactAnalysis && selectedNodeId && V3_H1_ENABLED) {
      setDetailTab('impact')
    }
  }, [impactAnalysis, selectedNodeId])

  const handleOpenProject = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false })
      if (!selected) return

      const path = selected as string
      const name = path.split('/').pop() ?? 'Project'

      // Reset analytics state for new/reopened project
      setArchitectureDetection(null)
      setImpactAnalysis(null)
      prevProjectId.current = null
      setLoading(true)
      setScanStartTime(Date.now())

      // Step 1: Try to reopen an already-indexed project (no re-scan needed)
      let reopenResult: ScanResult | null = null
      try {
        reopenResult = await openProjectByPath(path)
      } catch (reopenErr) {
        const reopenErrMsg = getErrorMessage(reopenErr)
        if (!reopenErrMsg.includes('No project found')) {
          setError(reopenErrMsg)
          setLoading(false)
          return
        }
      }

      if (reopenResult) {
        setScanResult(reopenResult)
        setStatus(reopenResult.status)
        if (reopenResult.projectId) {
          setProject(
            reopenResult.projectId,
            reopenResult.projectName || name,
            reopenResult.rootPath || path
          )
        }
        if (reopenResult.status === 'ready' && reopenResult.projectId) {
          try {
            const graph = await getGraph(reopenResult.projectId)
            const laid = buildLayout(graph)
            setGraphData(laid)
          } catch (graphErr) {
            setError(`Graph load failed: ${getErrorMessage(graphErr)}`)
            setLoading(false)
            return
          }
        }
        setLoading(false)
        return
      }

      // Step 2: Fresh scan for paths not yet indexed
      setProject(`proj-${Date.now()}`, name, path)
      setStatus('scanning')

      const result = await scanProject(path)
      setScanResult(result)
      setStatus(result.status)

      if (result.projectId) {
        setProject(result.projectId, result.projectName || name, result.rootPath || path)
      }

      if (result.status === 'ready' && result.projectId) {
        setLoading(true)
        try {
          const graph = await getGraph(result.projectId)
          const laid = buildLayout(graph)
          setGraphData(laid)
        } catch (e) {
          const msg = getErrorMessage(e)
          setError(`Graph load failed: ${msg}`)
        } finally {
          setLoading(false)
        }
      } else {
        setLoading(false)
      }
    } catch (err) {
      setError(getErrorMessage(err))
      setLoading(false)
    }
  }, [setProject, setStatus, setScanResult, setError, setGraphData, setLoading])

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
