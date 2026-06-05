// OutlineView — recursive tree render of OutlineItem hierarchy
// Integrates into DetailPanel to show a VS Code-like outline per node/file.

import { useState } from 'react'
import type { OutlineItem, OutlineItemKind } from '../../lib/types'

// ── Kind badge helpers ────────────────────────────────────────────────────

const KIND_ICONS: Record<OutlineItemKind, string> = {
  class: 'C',
  function: 'F',
  method: 'M',
  interface: 'I',
  type: 'T',
  enum: 'E',
  const: 'K', // const → K for "konstant"
  variable: 'V',
  module: 'D', // module → D for "definition/declaration"
  field: 'F',
  struct: 'S',
  impl: 'I',
  unknown: '?',
}

const KIND_LABELS: Record<OutlineItemKind, string> = {
  class: 'class',
  function: 'fn',
  method: 'method',
  interface: 'interface',
  type: 'type',
  enum: 'enum',
  const: 'const',
  variable: 'var',
  module: 'module',
  field: 'field',
  struct: 'struct',
  impl: 'impl',
  unknown: 'unknown',
}

function KindBadge({ kind }: { kind: OutlineItemKind }) {
  return (
    <span
      className="inline-block text-[9px] font-mono font-bold px-1 py-0.5 rounded"
      style={{ background: '#334155', color: '#94a3b8', minWidth: 20, textAlign: 'center' }}
      title={KIND_LABELS[kind] ?? kind}
    >
      {KIND_ICONS[kind] ?? '?'}
    </span>
  )
}

// ── Single outline item row ───────────────────────────────────────────────

function OutlineItemRow({ item, depth }: { item: OutlineItem; depth: number }) {
  const [collapsed, setCollapsed] = useState(false)
  const children = item.children ?? []
  const hasChildren = children.length > 0

  return (
    <li className="list-none">
      <div
        className="flex items-center gap-1.5 py-0.5 pr-1 rounded hover:bg-slate-700/40 cursor-default text-xs"
        style={{ paddingLeft: depth * 16 + 4 }}
      >
        {/* collapse toggle */}
        {hasChildren ? (
          <button
            onClick={() => setCollapsed((v) => !v)}
            className="w-4 h-4 flex items-center justify-center text-slate-500 hover:text-slate-300 flex-shrink-0"
            aria-label={collapsed ? 'expand' : 'collapse'}
          >
            {collapsed ? '▶' : '▼'}
          </button>
        ) : (
          <span className="w-4 h-4 flex-shrink-0" />
        )}

        <KindBadge kind={item.kind} />

        <span className="font-mono text-slate-200 truncate flex-1" title={item.name}>
          {item.name}
        </span>

        <span className="text-slate-600 font-mono text-[10px] flex-shrink-0">
          {item.lineStart}–{item.lineEnd}
        </span>
      </div>

      {/* children */}
      {hasChildren && !collapsed && (
        <ul className="m-0 p-0">
          {children.map((child) => (
            <OutlineItemRow key={child.id} item={child} depth={depth + 1} />
          ))}
        </ul>
      )}
    </li>
  )
}

// ── Main OutlineView component ────────────────────────────────────────────

interface OutlineViewProps {
  items: OutlineItem[]
  loading?: boolean
  error?: string | null
}

export function OutlineView({ items, loading = false, error = null }: OutlineViewProps) {
  // Loading
  if (loading) {
    return (
      <div className="flex items-center gap-2 py-2 text-xs text-slate-500">
        <span className="animate-pulse">◌</span>
        <span>Loading outline…</span>
      </div>
    )
  }

  // Error — show warning but preserve details
  if (error) {
    return (
      <div className="py-2 text-xs text-amber-400">
        <span>⚠ Outline unavailable</span>
        <span className="text-slate-500 ml-1">— details preserved below</span>
      </div>
    )
  }

  // Empty
  if (items.length === 0) {
    return <div className="py-2 text-xs text-slate-600 italic">No symbols detected</div>
  }

  // Tree
  return (
    <ul className="m-0 p-0">
      {items.map((item) => (
        <OutlineItemRow key={item.id} item={item} depth={0} />
      ))}
    </ul>
  )
}
