// DetailPanel — shows metadata of selected node
import { useEffect, useState } from 'react'
import { useSelectedNodeId, useGraphData } from '../../stores/graphStore'
import { getNodeDetails } from '../../lib/tauri-api'
import type { FileInfo } from '../../lib/types'
import { SymbolList } from './SymbolList'

export function DetailPanel() {
  const selectedNodeId = useSelectedNodeId()
  const graphData = useGraphData()
  const [details, setDetails] = useState<FileInfo | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!selectedNodeId) {
      setDetails(null)
      return
    }
    setLoading(true)
    setError(null)
    getNodeDetails(selectedNodeId)
      .then(setDetails)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)))
      .finally(() => setLoading(false))
  }, [selectedNodeId])

  if (!selectedNodeId) {
    return (
      <div className="h-full flex items-center justify-center text-slate-500 text-sm p-4">
        Select a node to see details
      </div>
    )
  }

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center text-slate-400 text-sm">Loading…</div>
    )
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center text-red-400 text-sm">
        Error: {error}
      </div>
    )
  }

  if (!details) return null

  const selectedNode = graphData?.nodes.find((n) => n.id === selectedNodeId)

  return (
    <div className="h-full overflow-y-auto bg-slate-900 text-slate-200 p-4 space-y-4">
      {/* Header */}
      <div>
        <h2 className="text-base font-semibold text-slate-100 truncate" title={details.name}>
          {details.name}
        </h2>
        <p className="text-xs text-slate-500 break-all" title={details.path}>
          {details.path}
        </p>
      </div>

      {/* Metadata */}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <div>
          <span className="text-slate-500">Type:</span>{' '}
          <span className="text-slate-300 capitalize">{selectedNode?.type ?? 'unknown'}</span>
        </div>
        <div>
          <span className="text-slate-500">Lines:</span>{' '}
          <span className="text-slate-300">{details.lines}</span>
        </div>
        <div>
          <span className="text-slate-500">Ext:</span>{' '}
          <span className="text-slate-300">{details.extension}</span>
        </div>
        <div>
          <span className="text-slate-500">Symbols:</span>{' '}
          <span className="text-slate-300">{details.symbols.length}</span>
        </div>
      </div>

      {/* Dependencies */}
      <div>
        <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wide mb-1">
          Dependencies
        </h3>
        {details.symbols.filter((s) => s.exports).length > 0 ? (
          <ul className="space-y-0.5">
            {details.symbols
              .filter((s) => s.exports)
              .slice(0, 20)
              .map((s) => (
                <li key={s.id} className="text-xs text-slate-300 flex items-center gap-1">
                  <span className="text-slate-600">▸</span>
                  <span className="font-mono">{s.name}</span>
                  <span className="text-slate-600">({s.kind})</span>
                </li>
              ))}
          </ul>
        ) : (
          <p className="text-xs text-slate-600">None exported</p>
        )}
      </div>

      {/* Symbols */}
      {details.symbols.length > 0 && <SymbolList symbols={details.symbols} />}
    </div>
  )
}
