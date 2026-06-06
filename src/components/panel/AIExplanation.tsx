// AI Explanation panel — shows AI summary for selected node
// Part of PR5b (AI UI)

import { useState, useEffect } from 'react'
import { Bot } from 'lucide-react'
import { explainNode } from '../../lib/tauri-api'
import { MarkdownView } from '../common/MarkdownView'
import { Spinner } from '../common/Spinner'
import { ErrorState } from '../common/ErrorState'

interface AIExplanationProps {
  nodeId: string | null
  projectId: string | null
  nodeLabel?: string
}

interface ExplanationState {
  status: 'idle' | 'loading' | 'ready' | 'error'
  data?: {
    summary: string
    details: string
    dependencies_note?: string
    role: string
  }
  error?: string
}

const QUICK_PROMPTS = [
  '¿Qué hace este componente?',
  'Muestra sus dependencias',
  'Explica su rol en la app',
]

export function AIExplanation({ nodeId, projectId, nodeLabel }: AIExplanationProps) {
  const [state, setState] = useState<ExplanationState>({ status: 'idle' })

  useEffect(() => {
    if (!nodeId || !projectId) {
      setState({ status: 'idle' })
      return
    }

    let cancelled = false
    setState({ status: 'loading' })

    explainNode(nodeId, projectId)
      .then((result) => {
        if (!cancelled) {
          setState({
            status: 'ready',
            data: {
              summary: result.summary,
              details: result.details,
              dependencies_note: result.dependencies_note,
              role: result.role,
            },
          })
        }
      })
      .catch((err: Error) => {
        if (!cancelled) {
          let msg = err.message || 'Error desconocido'
          if (msg.includes('401') || msg.includes('InvalidApiKey')) {
            msg = 'API key inválida o no configurada. Revisa Configuración.'
          } else if (msg.includes('429') || msg.includes('rate_limit')) {
            msg = 'Límite de peticiones alcanzado. Esperá unos segundos.'
          } else if (msg.includes('timeout') || msg.includes('TIMEOUT')) {
            msg = 'La respuesta tardó demasiado. Probá de nuevo.'
          } else if (msg.includes('ECONNREFUSED') || msg.includes('UNREACHABLE')) {
            msg = 'No se pudo conectar al proveedor de IA.'
          }
          setState({ status: 'error', error: msg })
        }
      })

    return () => {
      cancelled = true
    }
  }, [nodeId, projectId])

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

  if (state.status === 'loading') {
    return (
      <div className="flex flex-col items-center justify-center h-48 gap-3">
        <Spinner size="md" />
        <span className="text-xs text-text-muted">Analizando con IA...</span>
      </div>
    )
  }

  if (state.status === 'error') {
    return (
      <div className="p-4">
        <ErrorState
          message={state.error || 'Ocurrió un error al obtener la explicación.'}
          onRetry={() => {
            if (nodeId && projectId) {
              setState({ status: 'loading' })
            }
          }}
        />
      </div>
    )
  }

  if (state.status === 'ready' && state.data) {
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
        {state.data.role && (
          <span className="self-start px-2 py-0.5 bg-surface-inset text-accent-secondary text-xs rounded-full font-medium border border-border-subtle">
            {state.data.role}
          </span>
        )}

        {/* Summary */}
        <div>
          <h4 className="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">
            Resumen
          </h4>
          <MarkdownView content={state.data.summary} />
        </div>

        {/* Details */}
        {state.data.details && (
          <div>
            <h4 className="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">
              Detalles
            </h4>
            <MarkdownView content={state.data.details} />
          </div>
        )}

        {/* Dependencies note */}
        {state.data.dependencies_note && (
          <div className="border-t border-border-subtle pt-3">
            <h4 className="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2">
              Dependencias
            </h4>
            <MarkdownView content={state.data.dependencies_note} />
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
