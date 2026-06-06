// Chat input component with suggestions
// Part of PR5b (AI UI)

import { useState } from 'react'
import { Send } from 'lucide-react'

interface ChatInputProps {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
}

import { t } from '../../lib/i18n'

const DEFAULT_PLACEHOLDER = t('chat.inputPlaceholder')

const SUGGESTIONS = [
  '¿Qué archivos están relacionados con Auth?',
  'Explicá el flujo de datos de UserService',
  '¿Qué patrones de arquitectura usa este proyecto?',
]

export function ChatInput({
  onSend,
  disabled = false,
  placeholder = DEFAULT_PLACEHOLDER,
}: ChatInputProps) {
  const [value, setValue] = useState('')

  const handleSend = () => {
    const trimmed = value.trim()
    if (!trimmed || disabled) return
    onSend(trimmed)
    setValue('')
  }

  const handleKeyDown = (e: React.KeyboardEvent<unknown>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  return (
    <div className="flex flex-col gap-2">
      {/* Suggestions */}
      <div className="flex flex-wrap gap-1">
        {SUGGESTIONS.map((s) => (
          <button
            key={s}
            onClick={() => setValue(s)}
            disabled={disabled}
            className="px-2 py-0.5 text-xs bg-surface-inset text-accent-secondary rounded border border-accent-secondary/30 hover:bg-surface-hover hover:text-accent-secondary transition-colors disabled:opacity-50"
          >
            {s.length > 40 ? s.slice(0, 40) + '…' : s}
          </button>
        ))}
      </div>

      {/* Input row */}
      <div className="flex gap-2 items-end">
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          disabled={disabled}
          rows={2}
          className="flex-1 px-3 py-2 text-sm bg-surface-inset text-text-primary border border-border-subtle rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent disabled:bg-surface-base disabled:text-text-muted placeholder:text-text-muted"
          style={{ minHeight: '60px' }}
        />
        <button
          onClick={handleSend}
          disabled={disabled || !value.trim()}
          className="px-4 py-2 bg-accent-primary text-white text-sm font-medium rounded-lg hover:bg-accent-primary/80 active:bg-accent-primary/70 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
        >
          <Send size={16} />
          <span>{t('chat.send')}</span>
        </button>
      </div>

      <p className="text-xs text-text-muted">{t('chat.inputHint')}</p>
    </div>
  )
}
