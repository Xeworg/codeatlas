// SymbolList — collapsible list of symbols in a file
import { useState } from 'react'
import type { SymbolInfo } from '../../lib/types'

interface Props {
  symbols: SymbolInfo[]
}

const KIND_ICONS: Record<string, string> = {
  class: '🅒',
  function: '🄵',
  method: '🄵',
  interface: '🄸',
  type_alias: '🅃',
  enum: '🄴',
  variable: '🅅',
  const: '🅅',
  struct: '🅂',
  impl: '🄸',
  unknown: '?',
}

export function SymbolList({ symbols }: Props) {
  const [isOpen, setIsOpen] = useState(false)

  const exported = symbols.filter((s) => s.exports)
  const local = symbols.filter((s) => !s.exports)

  return (
    <div>
      <button
        onClick={() => setIsOpen((v) => !v)}
        className="flex items-center justify-between w-full text-xs font-semibold text-slate-400 uppercase tracking-wide hover:text-slate-300 mb-1"
      >
        <span>All Symbols ({symbols.length})</span>
        <span>{isOpen ? '▲' : '▼'}</span>
      </button>

      {isOpen && (
        <ul className="space-y-0.5 max-h-48 overflow-y-auto">
          {exported.length > 0 && (
            <>
              <li className="text-xs text-slate-600 font-medium px-1">Exported</li>
              {exported.map((s) => (
                <li key={s.id} className="text-xs text-slate-400 flex gap-2 pl-2">
                  <span>{KIND_ICONS[s.kind] ?? '?'}</span>
                  <span className="font-mono text-slate-300">{s.name}</span>
                </li>
              ))}
            </>
          )}
          {local.length > 0 && (
            <>
              <li className="text-xs text-slate-600 font-medium px-1 mt-1">Local</li>
              {local.slice(0, 20).map((s) => (
                <li key={s.id} className="text-xs text-slate-500 flex gap-2 pl-2">
                  <span>{KIND_ICONS[s.kind] ?? '?'}</span>
                  <span className="font-mono">{s.name}</span>
                </li>
              ))}
            </>
          )}
        </ul>
      )}
    </div>
  )
}
