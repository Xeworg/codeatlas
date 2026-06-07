// useAI hook — manages AI explanation and chat state
// Part of PR5b (AI UI), migrated to services in PR-8

import { useState, useCallback, useRef } from 'react'
import type { NodeExplanation, ChatResponse } from '../lib/types'
import { toApiError, toUserMessage } from '../lib/tauri-api'
import { explainNode, chat } from '../services/aiService'

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

  // Shared stale-result guard at hook level — persists across explain() calls
  // Each call gets a unique requestId; when a newer call starts, it marks
  // the previous call's requestId as stale so that response is discarded.
  const isStaleRef = useRef<{ requestId: number }>({ requestId: 0 })

  const explain = useCallback(async (options: ExplainOptions): Promise<NodeExplanation | null> => {
    const { nodeId, projectId } = options

    // Increment requestId — this marks any previous in-flight requests as stale
    const currentRequestId = isStaleRef.current.requestId + 1
    isStaleRef.current = { requestId: currentRequestId }

    setState((prev) => ({
      ...prev,
      explanation: { status: 'loading' },
    }))

    try {
      const result = await explainNode(nodeId, projectId)

      // Guard: if a newer request started, discard this stale response
      if (isStaleRef.current.requestId !== currentRequestId) {
        return null
      }

      setState((prev) => ({
        ...prev,
        explanation: { status: 'ready', data: result },
      }))
      return result
    } catch (err) {
      // Capture error locally — don't read hook state (stale-state race prevention)
      const apiErr = toApiError(err, 'UNREACHABLE')
      const userMsg = toUserMessage(apiErr)
      setState((prev) => ({
        ...prev,
        explanation: { status: 'error', error: userMsg },
      }))
      return null
    }
  }, [])

  const sendChat = useCallback(async (options: ChatOptions): Promise<ChatResponse> => {
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
      // Capture error locally — don't read hook state (stale-state race prevention)
      const apiErr = toApiError(err, 'UNREACHABLE')
      const userMsg = toUserMessage(apiErr)
      setState((prev) => ({
        ...prev,
        chat: { status: 'error', error: userMsg },
      }))
      // Throw so caller (ChatPanel) can catch it directly — no stale-state read needed
      throw new Error(userMsg)
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
