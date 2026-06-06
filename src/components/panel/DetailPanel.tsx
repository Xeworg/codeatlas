// DetailPanel — shows metadata of selected node
import { useEffect, useState } from 'react'
import { t } from '../../lib/i18n'
import { useSelectedNodeId, useGraphData } from '../../stores/graphStore'
import { getNodeDetails, getNodeOutline } from '../../lib/tauri-api'
import type { FileInfo, OutlineItem } from '../../lib/types'
import { SymbolList } from './SymbolList'
import { OutlineView } from './OutlineView'
import { ListTree, Link, Code, CornerDownRight } from 'lucide-react'

export function DetailPanel() {
  const selectedNodeId = useSelectedNodeId()
  const graphData = useGraphData()
  const [details, setDetails] = useState<FileInfo | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [outline, setOutline] = useState<OutlineItem[]>([])
  const [outlineLoading, setOutlineLoading] = useState(false)
  const [outlineError, setOutlineError] = useState<string | null>(null)

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

  // Load outline independently
  useEffect(() => {
    if (!selectedNodeId) {
      setOutline([])
      setOutlineError(null)
      return
    }
    setOutlineLoading(true)
    setOutlineError(null)
    getNodeOutline(selectedNodeId)
      .then(setOutline)
      .catch((e) => setOutlineError(e instanceof Error ? e.message : String(e)))
      .finally(() => setOutlineLoading(false))
  }, [selectedNodeId])

  if (!selectedNodeId) {
    return (
      <div className="h-full flex items-center justify-center text-text-muted text-sm p-4">
        {t('details.selectNode')}
      </div>
    )
  }

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center text-text-secondary text-sm">
        {t('common.loading')}
      </div>
    )
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center text-red-400 text-sm">
        {t('details.errorPrefix')} {error}
      </div>
    )
  }

  if (!details) return null

  const selectedNode = graphData?.nodes.find((n) => n.id === selectedNodeId)

  return (
    <div className="h-full overflow-y-auto bg-surface-base text-text-primary p-4 space-y-3">
      {/* Header — full-width, no card wrapper */}
      <div className="bg-surface-elevated rounded-lg border border-border-subtle p-3">
        <div className="flex items-center gap-2 mb-2">
          <span
            className={`px-1.5 py-0.5 text-[10px] font-bold uppercase rounded ${
              details.extension === '.ts' || details.extension === '.tsx'
                ? 'bg-blue-500/20 text-blue-400'
                : details.extension === '.js' || details.extension === '.jsx'
                  ? 'bg-yellow-500/20 text-yellow-400'
                  : details.extension === '.py'
                    ? 'bg-blue-400/20 text-blue-300'
                    : 'bg-surface-hover text-text-muted'
            }`}
          >
            {details.extension.replace('.', '') || '?'}
          </span>
          <h2
            className="text-sm font-semibold text-text-primary truncate flex-1"
            title={details.name}
          >
            {details.name}
          </h2>
        </div>
        <p className="text-xs text-text-muted break-all" title={details.path}>
          {details.path}
        </p>
      </div>

      {/* Metadata */}
      <div className="bg-surface-elevated rounded-lg border border-border-subtle p-3">
        <div className="flex items-center gap-2 mb-2">
          <Code size={14} className="text-text-muted" />
          <span className="text-xs font-semibold text-text-secondary uppercase tracking-wide">
            {t('details.metadata')}
          </span>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs">
          <div>
            <span className="text-text-muted">{t('details.type')}</span>{' '}
            <span className="text-text-secondary capitalize">
              {selectedNode?.type ?? t('details.unknownType')}
            </span>
          </div>
          <div>
            <span className="text-text-muted">{t('details.lines')}</span>{' '}
            <span className="text-text-secondary">{details.lines}</span>
          </div>
          <div>
            <span className="text-text-muted">{t('details.ext')}</span>{' '}
            <span className="text-text-secondary">{details.extension}</span>
          </div>
          <div>
            <span className="text-text-muted">{t('details.symbols')}</span>{' '}
            <span className="text-text-secondary">{details.symbols.length}</span>
          </div>
        </div>
      </div>

      {/* Outline */}
      <div className="bg-surface-elevated rounded-lg border border-border-subtle p-3">
        <div className="flex items-center gap-2 mb-2">
          <ListTree size={14} className="text-text-muted" />
          <h3 className="text-xs font-semibold text-text-secondary uppercase tracking-wide">
            {t('details.outline')}
          </h3>
        </div>
        <OutlineView items={outline} loading={outlineLoading} error={outlineError} />
      </div>

      {/* Dependencies */}
      <div className="bg-surface-elevated rounded-lg border border-border-subtle p-3">
        <div className="flex items-center gap-2 mb-2">
          <Link size={14} className="text-text-muted" />
          <h3 className="text-xs font-semibold text-text-secondary uppercase tracking-wide">
            {t('details.dependencies')}
          </h3>
        </div>
        {details.symbols.filter((s) => s.exports).length > 0 ? (
          <ul className="space-y-0.5">
            {details.symbols
              .filter((s) => s.exports)
              .slice(0, 20)
              .map((s) => (
                <li key={s.id} className="text-xs text-text-secondary flex items-center gap-1">
                  <CornerDownRight size={12} className="text-text-muted shrink-0" />
                  <span className="font-mono">{s.name}</span>
                  <span className="text-text-muted">({s.kind})</span>
                </li>
              ))}
          </ul>
        ) : (
          <p className="text-xs text-text-muted">{t('details.noneExported')}</p>
        )}
      </div>

      {/* Symbols */}
      {details.symbols.length > 0 && (
        <div className="bg-surface-elevated rounded-lg border border-border-subtle p-3">
          <div className="flex items-center gap-2 mb-3">
            <Code size={14} className="text-text-muted" />
            <span className="text-xs font-semibold text-text-secondary uppercase tracking-wide">
              {t('details.symbols')}
            </span>
          </div>
          <SymbolList symbols={details.symbols} />
        </div>
      )}
    </div>
  )
}
