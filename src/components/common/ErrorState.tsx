// ErrorState — error placeholder with a subtle red tinted card.
// Slice 4 (milestone 2): replaced ⚠️ emoji with the Lucide
// `AlertTriangle` icon, and moved away from the bright red defaults
// to a refined red-tinted disc that matches the reference's restrained
// danger styling.
import type { ReactNode } from 'react'

interface ErrorStateProps {
  icon?: ReactNode
  message: string
  onRetry?: () => void
  actionLabel?: string
}

export function ErrorState({
  icon,
  message,
  onRetry,
  actionLabel,
}: ErrorStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-4 p-10 text-center">
      <div
        aria-hidden
        className="w-14 h-14 rounded-md flex items-center justify-center bg-surface-elevated border border-red-500/20 text-red-400 shadow-panel"
      >
        {icon ?? (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width={24}
            height={24}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.75}
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M21.73 18l-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z" />
            <path d="M12 9v4" />
            <path d="M12 17h.01" />
          </svg>
        )}
      </div>
      <div className="space-y-1.5 max-w-xs">
        <h3 className="text-sm font-semibold text-red-300">Error</h3>
        <p className="text-xs text-text-muted leading-relaxed">{message}</p>
      </div>
      {onRetry && (
        <button
          onClick={onRetry}
          className="mt-1 px-3.5 py-1.5 text-xs font-medium text-red-300 border border-red-500/30 rounded-sm hover:bg-red-500/10 hover:border-red-500/50 transition-colors"
        >
          {actionLabel ?? 'Reintentar'}
        </button>
      )}
    </div>
  )
}
