// useAI hook — manages AI explanation and chat state
// Part of PR5b (AI UI)

import { useState, useCallback } from 'react'
import type { NodeExplanation, ChatResponse } from '../lib/types'
import { explainNode, chat } from '../lib/tauri-api'

interface AIState {
  explanation: {
    status: 'idle' | 'loading' | 'ready' | 'error'
    data?: NodeExplanation
    error?: string
  }
  chat: {
    status: 'idle' | 'sending' | 'ready' | 'error'
    error?: string
  }
}

interface ExplainOptions {
  nodeId: string
  projectId: string
}

interface ChatOptions {
  projectId: string
  message: string
  history: { id: string; role: string; content: string; timestamp: string }[]
  contextNodeIds?: string[]
}

export function useAI() {
  const [state, setState] = useState<AIState>({
    explanation: { status: 'idle' },
    chat: { status: 'idle' },
  })

  const explain = useCallback(async (options: ExplainOptions): Promise<NodeExplanation | null> => {
    const { nodeId, projectId } = options
    setState((prev) => ({
      ...prev,
      explanation: { status: 'loading' },
    }))

    try {
      const result = await explainNode(nodeId, projectId)
      setState((prev) => ({
        ...prev,
        explanation: { status: 'ready', data: result },
      }))
      return result
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error desconocido'
      setState((prev) => ({
        ...prev,
        explanation: { status: 'error', error: msg },
      }))
      return null
    }
  }, [])

  const sendChat = useCallback(async (options: ChatOptions): Promise<ChatResponse | null> => {
    const { projectId, message, history, contextNodeIds } = options
    setState((prev) => ({
      ...prev,
      chat: { status: 'sending' },
    }))

    try {
      const result = await chat(projectId, message, history, contextNodeIds)
      setState((prev) => ({
        ...prev,
        chat: { status: 'ready' },
      }))
      return result
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error desconocido'
      setState((prev) => ({
        ...prev,
        chat: { status: 'error', error: msg },
      }))
      return null
    }
  }, [])

  const resetExplanation = useCallback(() => {
    setState((prev) => ({
      ...prev,
      explanation: { status: 'idle' },
    }))
  }, [])

  const resetChat = useCallback(() => {
    setState((prev) => ({
      ...prev,
      chat: { status: 'idle' },
    }))
  }, [])

  return {
    state,
    explain,
    sendChat,
    resetExplanation,
    resetChat,
    isExplanationLoading: state.explanation.status === 'loading',
    isChatSending: state.chat.status === 'sending',
  }
}