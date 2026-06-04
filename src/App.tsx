// App — main entry, wires all panels together
// PR2 (v3): integrates AnalyticsViewSelector, ArchitectureCard, ImpactPanel, InsightsPanel
// Gate T5.6: components wire-ready since v2, now fully integrated into main flow

import { useState, useCallback, useEffect, useRef } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
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
import {
  AnalyticsViewSelector,
  ArchitectureCard,
  ImpactPanel,
  InsightsPanel,
} from './components/analytics'
import { useProjectStore, useScanStatus } from './stores/projectStore'
import { useGraphStore, useSelectedNodeId } from './stores/graphStore'
import {
  scanProject,
  getGraph,
  getErrorMessage,
  getArchitectureDetection,
  getImpactAnalysis,
  getGraphInsights,
} from './lib/tauri-api'
import { buildLayout } from './lib/graph-layout'
import { V3_H1_ENABLED } from './stores/featureFlags'
import type { ArchitectureDetectionResult, ImpactAnalysisResult, GraphInsights } from './lib/types'

type DetailTab = 'details' | 'ai' | 'chat' | 'impact'

const V2_DETAIL_TABS: { id: DetailTab; label: string; icon: string }[] = [
  { id: 'details', label: 'Detalles', icon: '📋' },
  { id: 'ai', label: 'IA', icon: '🤖' },
  { id: 'chat', label: 'Chat', icon: '💬' },
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

  // ── PR2 analytics state ────────────────────────────────────────────────
  const [architectureDetection, setArchitectureDetection] =
    useState<ArchitectureDetectionResult | null>(null)
  const [impactAnalysis, setImpactAnalysis] = useState<ImpactAnalysisResult | null>(null)
  const [graphInsights, setGraphInsights] = useState<GraphInsights | null>(null)

  // Track previous projectId to re-fetch analytics on project change
  const prevProjectId = useRef<string | null>(null)

  // ── T2.2: Fetch architecture detection when project is ready ─────────────
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

  // ── T2.2: Fetch graph insights when project is ready ────────────────────
  useEffect(() => {
    if (!V3_H1_ENABLED) return
    if (status !== 'ready' || !projectId) return

    setGraphInsights(null)
    getGraphInsights(projectId)
      .then(setGraphInsights)
      .catch(() => setGraphInsights(null))
  }, [status, projectId])

  // ── T2.3: Fetch impact analysis when node is selected ───────────────────
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

  // Auto-select Impact tab when impact analysis arrives and a node is selected
  useEffect(() => {
    if (impactAnalysis && selectedNodeId && V3_H1_ENABLED) {
      setDetailTab('impact')
    }
  }, [impactAnalysis, selectedNodeId])

  // Auto-select AI tab when a node is selected (v2 behavior, preserved)
  useEffect(() => {
    if (selectedNodeId && status === 'ready' && detailTab === 'details') {
      // Only switch if not already in a non-details tab to avoid hijacking
    }
  }, [selectedNodeId, status])

  const handleOpenProject = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false })
      if (!selected) return

      const path = selected as string
      const name = path.split('/').pop() ?? 'Project'

      // Reset analytics state for new project
      setArchitectureDetection(null)
      setImpactAnalysis(null)
      setGraphInsights(null)
      prevProjectId.current = null

      // Temporary project marker while scan runs
      setProject(`proj-${Date.now()}`, name, path)
      setStatus('scanning')
      setScanStartTime(Date.now())
      setLoading(true)

      const result = await scanProject(path)
      setScanResult(result)
      setStatus(result.status)

      // Promote canonical backend project id for all subsequent hooks/commands
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

    switch (detailTab) {
      case 'ai':
        return (
          <AIExplanation
            nodeId={selectedNodeId}
            projectId={projectId}
            nodeLabel={
              selectedNodeId
                ? graphData?.nodes.find((n) => n.id === selectedNodeId)?.label
                : undefined
            }
          />
        )
      case 'chat':
        return (
          <ChatPanel
            projectId={projectId}
            contextNodeIds={selectedNodeId ? [selectedNodeId] : []}
          />
        )
      case 'details':
      default:
        return <DetailPanel />
    }
  }

  const detailTabs = V3_H1_ENABLED
    ? [...V2_DETAIL_TABS, { id: 'impact' as DetailTab, label: 'Impacto', icon: '💥' }]
    : V2_DETAIL_TABS

  const mainContent = () => {
    if (error) return <ErrorState message={error} onRetry={handleOpenProject} actionLabel="Retry" />

    if (status === 'idle') {
      return (
        <EmptyState
          icon="📂"
          title="No project"
          description="Open a project to explore its architecture"
          action={{ label: 'Open project', onClick: handleOpenProject }}
        />
      )
    }

    if (status === 'scanning' || status === 'building_graph') {
      return (
        <div className="flex items-center justify-center h-full">
          <div className="text-center">
            <Spinner size="lg" />
            <p className="mt-3 text-sm text-slate-400">
              {status === 'scanning' ? 'Scanning files…' : 'Building graph…'}
            </p>
          </div>
        </div>
      )
    }

    if (status === 'ready') {
      return (
        <div className="flex flex-col h-full overflow-hidden">
          {/* T2.1: AnalyticsViewSelector toolbar — only when v3_h1 enabled */}
          {V3_H1_ENABLED && <AnalyticsViewSelector />}

          {/* T2.2: ArchitectureCard — shown when detection is available */}
          {V3_H1_ENABLED && architectureDetection && (
            <div className="px-4 py-2 bg-slate-900 border-b border-slate-700 flex-shrink-0">
              <ArchitectureCard detection={architectureDetection} />
            </div>
          )}

          {/* T2.4: InsightsPanel — shown below graph when insights are available */}
          {V3_H1_ENABLED && graphInsights ? (
            <div className="h-52 border-t border-slate-700 flex-shrink-0 flex flex-col overflow-hidden bg-slate-900">
              <InsightsPanel insights={graphInsights} />
            </div>
          ) : null}

          {/* Main graph area */}
          <div className="flex-1 relative overflow-hidden">
            <GraphView />
            <SearchOverlay />
          </div>

          {/* Detail panel — tabs include Impact when v3_h1 */}
          {selectedNodeId && (
            <div className="h-72 border-t border-slate-700 overflow-hidden flex-shrink-0 flex flex-col bg-white">
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

    return <EmptyState icon="?" title="Unknown state" />
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
    >
      {mainContent()}
    </AppShell>
  )
}

export default App
