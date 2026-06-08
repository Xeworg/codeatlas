// AI Explanation panel — shows AI summary for selected node
// Part of PR5b (AI UI), migrated to useAI hook in PR-8

import { useEffect } from 'react'
import { Bot } from 'lucide-react'
import { useAI } from '../../hooks/useAI'
import { MarkdownView } from '../common/MarkdownView'
import { Spinner } from '../common/Spinner'
import { ErrorState } from '../common/ErrorState'

interface AIExplanationProps {
  nodeId: string | null
  projectId: string | null
  nodeLabel?: string
}

const QUICK_PROMPTS = [
  '¿Qué hace este componente?',
  'Muestra sus dependencias',
  'Explica su rol en la app',
]

export function AIExplanation({ nodeId, projectId, nodeLabel }: AIExplanationProps) {
  const { state: aiState, explain, isExplanationLoading } = useAI()

  useEffect(() => {
    if (!nodeId || !projectId) return

    explain({ nodeId, projectId })
  }, [nodeId, projectId, explain])

  if (!nodeId || !projectId) {
    return (
      <div className="flex flex-col items-center justify-center h-48 text-text-muted">
        <Bot size={28} className="mb-2 opacity-60" />
        <p className="text-sm text-center px-4">
          Seleccioná un nodo en el grafo para ver su explicación IA
        </p>
      </div>
    )
  }

  if (isExplanationLoading || aiState.explanation.status === 'loading') {
    return (
      <div className="flex flex-col items-center justify-center h-48 gap-3">
        <Spinner size="md" />
        <span className="text-xs text-text-muted">Analizando con IA...</span>
      </div>
    )
  }

  if (aiState.explanation.status === 'error') {
    return (
      <div className="p-4">
        <ErrorState
          message={aiState.explanation.error || 'Ocurrió un error al obtener la explicación.'}
          onRetry={() => {
            if (nodeId && projectId) {
              explain({ nodeId, projectId })
            }
          }}
        />
      </div>
    )
  }

  if (aiState.explanation.status === 'ready' && aiState.explanation.data) {
    const { data } = aiState.explanation
    return (
      <div className="flex flex-col gap-4 p-4">
        {/* Header */}
        <div className="flex items-center gap-2">
          <Bot size={18} className="text-text-muted" />
          <div>
            <h3 className="text-sm font-semibold text-text-primary">Explicación IA</h3>
            {nodeLabel && (
              <p className="text-xs text-text-muted truncate max-w-[200px]">{nodeLabel}</p>
            )}
          </div>
        </div>

        {/* Role badge */}
        {data.role && (
          <span className="self-start px-2 py-0.5 bg-surface-inset text-accent-secondary text-xs rounded-full font-medium border border-border-subtle">
            {data.role}
          </span>
        )}

        {/* Summary */}
        <div>
          <h4 className="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">
            Resumen
          </h4>
          <MarkdownView content={data.summary} />
        </div>

        {/* Details */}
        {data.details && (
          <div>
            <h4 className="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">
              Detalles
            </h4>
            <MarkdownView content={data.details} />
          </div>
        )}

        {/* Dependencies note */}
        {data.dependenciesNote && (
          <div className="border-t border-border-subtle pt-3">
            <h4 className="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">
              Dependencias
            </h4>
            <MarkdownView content={data.dependenciesNote} />
          </div>
        )}

        {/* Quick prompts */}
        <div className="border-t border-border-subtle pt-3">
          <p className="text-xs text-text-muted mb-2">Consultas rápidas:</p>
          <div className="flex flex-wrap gap-1">
            {QUICK_PROMPTS.map((prompt) => (
              <button
                key={prompt}
                onClick={() => {
                  /* pass to chat context */
                }}
                className="px-2 py-1 text-xs bg-surface-elevated text-accent-primary rounded border border-border-subtle hover:bg-surface-hover transition-colors"
              >
                {prompt}
              </button>
            ))}
          </div>
        </div>
      </div>
    )
  }

  return null
}
