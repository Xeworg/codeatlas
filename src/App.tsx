// App — main entry, wires all panels together
import { useState, useCallback, useEffect } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { AppShell } from './components/layout/AppShell'
import { Sidebar } from './components/layout/Sidebar'
import { EmptyState } from './components/common/EmptyState'
import { ErrorState } from './components/common/ErrorState'
import { Spinner } from './components/common/Spinner'
import { GraphView } from './components/graph/GraphView'
import { SearchOverlay } from './components/graph/SearchOverlay'
import { DetailPanel } from './components/panel/DetailPanel'
import { useProjectStore, useScanStatus } from './stores/projectStore'
import { useGraphStore, useSelectedNodeId } from './stores/graphStore'
import { scanProject, getGraph } from './lib/tauri-api'
import { buildLayout } from './lib/graph-layout'

function App() {
  const status = useScanStatus()
  const selectedNodeId = useSelectedNodeId()
  const { scanResult, projectName, error, setProject, setScanResult, setStatus, setError } =
    useProjectStore()
  const { selectNode, setGraphData, setLoading } = useGraphStore()

  const [scanStartTime, setScanStartTime] = useState<number | null>(null)
  const [showDetails, setShowDetails] = useState(false)

  useEffect(() => {
    setShowDetails(!!selectedNodeId && status === 'ready')
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
          {showDetails && (
            <div className="h-64 border-t border-slate-700 overflow-hidden flex-shrink-0">
              <DetailPanel />
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
