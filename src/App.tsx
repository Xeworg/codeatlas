// App — main entry, wires all panels together
// Part of PR5b (AI UI) — integrates AIExplanation and ChatPanel

import { useState, useCallback, useEffect } from 'react'
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
import { useProjectStore, useScanStatus } from './stores/projectStore'
import { useGraphStore, useSelectedNodeId } from './stores/graphStore'
import { scanProject, getGraph } from './lib/tauri-api'
import { buildLayout } from './lib/graph-layout'

type DetailTab = 'details' | 'ai' | 'chat'

const DETAIL_TABS: { id: DetailTab; label: string; icon: string }[] = [
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

  // Auto-select AI tab when a node is selected
  useEffect(() => {
    if (selectedNodeId && status === 'ready') {
      setDetailTab('ai')
    }
  }, [selectedNodeId, status])

  const handleOpenProject = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false })
      if (!selected) return

      const path = selected as string
      const name = path.split('/').pop() ?? 'Project'
      const newProjectId = `proj-${Date.now()}`

      setProject(newProjectId, name, path)
      setStatus('scanning')
      setScanStartTime(Date.now())
      setLoading(true)

      const result = await scanProject(path)
      setScanResult(result)
      setStatus(result.status)

      if (result.status === 'ready' && result.project_id) {
        setLoading(true)
        try {
          const graph = await getGraph(result.project_id)
          const laid = buildLayout(graph)
          setGraphData(laid)
        } catch {
          setGraphData({
            nodes: [],
            edges: [],
            project_id: result.project_id,
            generated_at: new Date().toISOString(),
          })
        } finally {
          setLoading(false)
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [setProject, setStatus, setScanResult, setError, setGraphData, setLoading])

  const handleSelectFile = useCallback(
    (fileId: string) => {
      selectNode(fileId)
    },
    [selectNode]
  )

  const renderDetailContent = () => {
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
          <div className="flex-1 relative overflow-hidden">
            <GraphView />
            <SearchOverlay />
          </div>
          {selectedNodeId && (
            <div className="h-72 border-t border-slate-700 overflow-hidden flex-shrink-0 flex flex-col bg-white">
              <TabSwitcher
                tabs={DETAIL_TABS}
                activeTab={detailTab}
                onChange={(id) => setDetailTab(id as DetailTab)}
              />
              <div className="flex-1 overflow-y-auto">
                {renderDetailContent()}
              </div>
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