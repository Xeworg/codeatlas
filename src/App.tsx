import { useState, useCallback } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { AppShell } from './components/layout/AppShell'
import { Sidebar } from './components/layout/Sidebar'
import { EmptyState } from './components/common/EmptyState'
import { ErrorState } from './components/common/ErrorState'
import { Spinner } from './components/common/Spinner'
import { useProjectStore, useScanStatus } from './stores/projectStore'
import { useGraphStore, useSelectedNodeId } from './stores/graphStore'
import { scanProject } from './lib/tauri-api'

function App() {
  const status = useScanStatus()
  const selectedNodeId = useSelectedNodeId()
  const { scanResult, projectName, error, setProject, setScanResult, setStatus, setError } =
    useProjectStore()
  const { selectNode, setGraphData } = useGraphStore()

  const [scanStartTime, setScanStartTime] = useState<number | null>(null)

  const handleOpenProject = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false })
      if (!selected) return

      const path = selected as string
      const name = path.split('/').pop() ?? 'Proyecto'
      const projectId = `proj-${Date.now()}`

      setProject(projectId, name, path)
      setStatus('scanning')
      setScanStartTime(Date.now())

      const result = await scanProject(path)

      setScanResult(result)
      setStatus(result.status)

      if (result.status === 'ready' && result.project_id) {
        setGraphData({
          nodes: [],
          edges: [],
          project_id: result.project_id,
          generated_at: new Date().toISOString(),
        })
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [setProject, setStatus, setScanResult, setError, setGraphData])

  const handleSelectFile = useCallback(
    (fileId: string) => {
      selectNode(fileId)
    },
    [selectNode]
  )

  const handleSidebarSearch = useCallback((_query: string) => {
    // TODO: wire to search_nodes in PR4b
  }, [])

  // Content states
  const mainContent = () => {
    if (error)
      return <ErrorState message={error} onRetry={handleOpenProject} actionLabel="Reintentar" />

    if (status === 'idle') {
      return (
        <EmptyState
          icon="📂"
          title="Sin proyecto"
          description="Abrí un proyecto para explorar su arquitectura"
          action={{ label: 'Abrir proyecto', onClick: handleOpenProject }}
        />
      )
    }

    if (status === 'scanning' || status === 'building_graph') {
      return (
        <div className="flex items-center justify-center h-full">
          <Spinner size="lg" />
        </div>
      )
    }

    if (status === 'ready') {
      return (
        <div className="flex items-center justify-center h-full text-gray-500 text-sm">
          {scanResult
            ? `${scanResult.files_count} archivos cargados — panel de grafo viene en PR4b`
            : 'Proyecto listo'}
        </div>
      )
    }

    return <EmptyState icon="?" title="Estado desconocido" />
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
          onSearch={handleSidebarSearch}
        />
      }
    >
      {mainContent()}
    </AppShell>
  )
}

export default App
