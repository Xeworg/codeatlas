// EmptyState — polished empty placeholder for views without data.
// Slice 4 (milestone 2): moved from bright blue defaults to the dark
// reference palette. `icon` is now a ReactNode so callers can pass a
// Lucide component instead of an emoji. The icon sits in a soft
// elevated disc tinted with the violet accent to echo the reference.
import type { ReactNode } from 'react'

interface EmptyStateProps {
  icon?: ReactNode
  title: string
  description?: string
  action?: {
    label: string
    onClick: () => void
  }
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-4 p-10 text-center">
      {icon && (
        <div
          aria-hidden
          className="w-14 h-14 rounded-md flex items-center justify-center bg-surface-elevated border border-border-subtle text-accent-secondary shadow-panel"
        >
          {icon}
        </div>
      )}
      <div className="space-y-1.5 max-w-xs">
        <h3 className="text-sm font-semibold text-text-primary">{title}</h3>
        {description && <p className="text-xs text-text-muted leading-relaxed">{description}</p>}
      </div>
      {action && (
        <button
          onClick={action.onClick}
          className="mt-1 px-3.5 py-1.5 text-xs font-medium text-text-primary border border-border-subtle rounded-sm hover:border-border-strong hover:bg-surface-hover transition-colors"
        >
          {action.label}
        </button>
      )}
    </div>
  )
}
