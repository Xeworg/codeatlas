interface ErrorStateProps {
  message: string
  onRetry?: () => void
  actionLabel?: string
}

export function ErrorState({ message, onRetry, actionLabel }: ErrorStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 p-8 text-center">
      <span className="text-4xl opacity-50">⚠️</span>
      <h3 className="text-base font-medium text-red-400">Error</h3>
      <p className="text-sm text-gray-400 max-w-xs">{message}</p>
      {onRetry && (
        <button
          onClick={onRetry}
          className="mt-2 px-4 py-2 text-sm bg-red-600 hover:bg-red-500 text-white rounded-md transition-colors"
        >
          {actionLabel ?? 'Reintentar'}
        </button>
      )}
    </div>
  )
}
