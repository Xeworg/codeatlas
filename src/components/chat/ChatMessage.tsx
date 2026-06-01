// Chat message bubble component
// Part of PR5b (AI UI)

import type { ChatMessage as ChatMessageType } from '../../lib/types'
import { MarkdownView } from '../common/MarkdownView'

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
            ? 'bg-blue-600 text-white rounded-br-md'
            : isAssistant
            ? 'bg-gray-100 text-gray-800 rounded-bl-md'
            : 'bg-gray-50 text-gray-500 italic text-sm'
        }`}
      >
        {/* Role badge for assistant */}
        {isAssistant && (
          <div className="text-xs text-purple-600 font-medium mb-1 flex items-center gap-1">
            <span>🤖</span>
            <span>CodeAtlas</span>
          </div>
        )}

        {/* Content */}
        <div className={isUser ? 'text-white' : 'text-gray-800'}>
          <MarkdownView content={message.content} />
        </div>

        {/* Timestamp */}
        {message.timestamp && (
          <p
            className={`text-xs mt-1.5 ${
              isUser ? 'text-blue-200' : 'text-gray-400'
            }`}
          >
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