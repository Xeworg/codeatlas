// Chat panel — contextual AI chat with project context
// Part of PR5b (AI UI), migrated to useAI hook in PR-8

import { useState, useRef, useEffect } from 'react'
import { Sparkles, MoreHorizontal, MessageSquare } from 'lucide-react'
import { useAI } from '../../hooks/useAI'
import { ChatMessage } from './ChatMessage'
import { ChatInput } from './ChatInput'
import { Spinner } from '../common/Spinner'
import { ErrorState } from '../common/ErrorState'
import { t } from '../../lib/i18n'

interface ChatPanelProps {
  projectId: string | null
  contextNodeIds?: string[]
  onError?: (error: string) => void
}

interface MessageState {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: string
}

export function ChatPanel({ projectId, contextNodeIds = [], onError }: ChatPanelProps) {
  const { state: aiState, sendChat, isChatSending } = useAI()
  const [messages, setMessages] = useState<MessageState[]>([])
  const [errorMsg, setErrorMsg] = useState<string | null>(null)
  const messagesEndRef = useRef<HTMLDivElement | null>(null)

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, aiState.chat.status])

  const handleSend = async (content: string) => {
    if (!projectId) {
      setErrorMsg(t('chat.noProjectLoaded'))
      return
    }
    if (isChatSending || aiState.chat.status === 'sending') return

    const userMessage: MessageState = {
      id: `user-${Date.now()}`,
      role: 'user',
      content,
      timestamp: new Date().toISOString(),
    }

    setMessages((prev) => [...prev, userMessage])
    setErrorMsg(null)

    try {
      const result = await sendChat({
        projectId,
        message: content,
        history: messages,
        contextNodeIds,
      })

      const assistantMessage: MessageState = {
        id: result.message.id || `assistant-${Date.now()}`,
        role: result.message.role,
        content: result.message.content,
        timestamp: result.message.timestamp || new Date().toISOString(),
      }
      setMessages((prev) => [...prev, assistantMessage])
    } catch (err) {
      // Catch the thrown error directly — no stale-state read needed
      const errMsg = err instanceof Error ? err.message : 'Error de IA'
      setErrorMsg(errMsg)
      onError?.(errMsg)
    }
  }

  const handleRetry = () => {
    setErrorMsg(null)
  }

  return (
    <div className="flex flex-col min-h-0 h-full">
      {/* Header */}
      <div className="flex items-center gap-2 px-4 py-3 border-b border-border-subtle bg-surface-elevated flex-shrink-0">
        <Sparkles size={16} className="text-accent-secondary" />
        <h2 className="text-sm font-semibold text-text-primary flex-1">{t('chat.title')}</h2>
        <button
          onClick={() => {}}
          className="p-1 text-text-muted hover:text-text-secondary cursor-default"
          title={t('chat.menuTooltip')}
        >
          <MoreHorizontal size={14} />
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3 min-h-0">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full text-text-muted py-8">
            <MessageSquare size={32} className="mb-3 text-text-muted opacity-60" />
            <p className="text-sm text-center px-4">{t('chat.emptyStateHint')}</p>
            <p className="text-xs text-text-muted mt-1">{t('chat.emptyStateExample')}</p>
          </div>
        )}

        {messages.map((msg) => (
          <ChatMessage key={msg.id} message={msg} />
        ))}

        {(isChatSending || aiState.chat.status === 'sending') && (
          <div className="flex items-center gap-2 text-text-muted">
            <Spinner size="sm" />
            <span className="text-xs">{t('chat.writing')}</span>
          </div>
        )}

        {aiState.chat.status === 'error' && errorMsg && (
          <div className="p-4">
            <ErrorState message={errorMsg} onRetry={handleRetry} />
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-border-subtle bg-surface-inset flex-shrink-0">
        {projectId ? (
          <ChatInput
            onSend={handleSend}
            disabled={isChatSending || aiState.chat.status === 'sending'}
          />
        ) : (
          <div className="text-center text-sm text-text-muted py-2">
            {t('chat.openProjectToChat')}
          </div>
        )}
      </div>
    </div>
  )
}
