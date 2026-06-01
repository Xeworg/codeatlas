// Chat input component with suggestions
// Part of PR5b (AI UI)

import { useState } from 'react'

interface ChatInputProps {
  onSend: (message: string) => void
  disabled?: boolean
  placeholder?: string
}

const DEFAULT_PLACEHOLDER = 'Escribí tu pregunta sobre el proyecto...'

const SUGGESTIONS = [
  '¿Qué archivos están relacionados con Auth?',
  'Explicá el flujo de datos de UserService',
  '¿Qué patrones de arquitectura usa este proyecto?',
]

export function ChatInput({ onSend, disabled = false, placeholder = DEFAULT_PLACEHOLDER }: ChatInputProps) {
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
            className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded border border-gray-200 hover:bg-gray-200 hover:text-gray-800 transition-colors disabled:opacity-50"
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
          className="flex-1 px-3 py-2 text-sm border border-gray-300 rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:text-gray-400 placeholder:text-gray-400"
          style={{ minHeight: '60px' }}
        />
        <button
          onClick={handleSend}
          disabled={disabled || !value.trim()}
          className="px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 active:bg-blue-800 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
        >
          <span>Enviar</span>
          <span>→</span>
        </button>
      </div>

      <p className="text-xs text-gray-400">Enter para enviar · Shift+Enter para nueva línea</p>
    </div>
  )
}