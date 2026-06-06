// SymbolList — collapsible list of symbols in a file
import { useState } from 'react'
import { ChevronDown, ChevronUp } from 'lucide-react'
import type { SymbolInfo } from '../../lib/types'

interface Props {
  symbols: SymbolInfo[]
}

const KIND_ICONS: Record<string, React.ReactNode> = {
  class: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      C
    </span>
  ),
  function: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      F
    </span>
  ),
  method: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      M
    </span>
  ),
  interface: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      I
    </span>
  ),
  type_alias: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      T
    </span>
  ),
  enum: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      E
    </span>
  ),
  variable: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      V
    </span>
  ),
  const: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      K
    </span>
  ),
  struct: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      S
    </span>
  ),
  impl: (
    <span
      className="inline-block text-[9px] font-mono font-bold bg-surface-inset text-text-secondary px-0.5 rounded"
      style={{ minWidth: 14, textAlign: 'center' }}
    >
      I
    </span>
  ),
  unknown: <span className="inline-block text-[9px] font-mono text-text-muted">?</span>,
}

export function SymbolList({ symbols }: Props) {
  const [isOpen, setIsOpen] = useState(false)

  const exported = symbols.filter((s) => s.exports)
  const local = symbols.filter((s) => !s.exports)

  return (
    <div>
      <button
        onClick={() => setIsOpen((v) => !v)}
        className="flex items-center justify-between w-full text-xs font-semibold text-text-muted uppercase tracking-wide hover:text-text-secondary mb-1"
      >
        <span>All Symbols ({symbols.length})</span>
        {isOpen ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
      </button>

      {isOpen && (
        <ul className="space-y-0.5 max-h-48 overflow-y-auto">
          {exported.length > 0 && (
            <>
              <li className="text-xs text-text-muted font-medium px-1">Exported</li>
              {exported.map((s) => (
                <li
                  key={s.id}
                  className="text-xs text-text-secondary flex items-center gap-1.5 pl-2"
                >
                  {KIND_ICONS[s.kind] ?? <span className="text-text-muted">?</span>}
                  <span className="font-mono text-text-primary">{s.name}</span>
                </li>
              ))}
            </>
          )}
          {local.length > 0 && (
            <>
              <li className="text-xs text-text-muted font-medium px-1 mt-1">Local</li>
              {local.slice(0, 20).map((s) => (
                <li key={s.id} className="text-xs text-text-muted flex items-center gap-1.5 pl-2">
                  {KIND_ICONS[s.kind] ?? <span className="text-text-muted">?</span>}
                  <span className="font-mono text-text-primary">{s.name}</span>
                </li>
              ))}
            </>
          )}
        </ul>
      )}
    </div>
  )
}
