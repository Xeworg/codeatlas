// Chat message bubble component
// Part of PR5b (AI UI)

import type { ChatMessage as ChatMessageType } from '../../lib/types'
import { MarkdownView } from '../common/MarkdownView'
import { Bot } from 'lucide-react'

interface ChatMessageProps {
  message: ChatMessageType
}

export function ChatMessage({ message }: ChatMessageProps) {
  const isUser = message.role === 'user'
  const isAssistant = message.role === 'assistant'

  return (
    <div className={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
      <div
        className={`max-w-[85%] rounded-2xl px-4 py-2.5 ${
          isUser
            ? 'bg-accent-primary text-white rounded-br-md'
            : isAssistant
              ? 'bg-surface-elevated text-text-primary rounded-bl-md border border-border-subtle'
              : 'bg-surface-inset text-text-muted italic text-sm'
        }`}
      >
        {/* Role badge for assistant */}
        {isAssistant && (
          <div className="text-xs text-accent-secondary font-medium mb-1 flex items-center gap-1">
            <Bot size={12} />
            <span>CodeAtlas</span>
          </div>
        )}

        {/* Content */}
        <div className={isUser ? 'text-white' : 'text-text-primary'}>
          <MarkdownView content={message.content} />
        </div>

        {/* Timestamp */}
        {message.timestamp && (
          <p className={`text-xs mt-1.5 ${isUser ? 'text-accent-primary/60' : 'text-text-muted'}`}>
            {new Date(message.timestamp).toLocaleTimeString('es-AR', {
              hour: '2-digit',
              minute: '2-digit',
            })}
          </p>
        )}
      </div>
    </div>
  )
}
