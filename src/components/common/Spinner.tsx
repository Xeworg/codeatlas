// Spinner — quiet loading indicator with the violet accent
// Slice 4 (milestone 2): aligned to the dark reference palette.
// The track uses the subtle border token so the spinner feels
// embedded in the surface; the rotating head uses the violet
// accent to match the rest of the active-state styling.
interface SpinnerProps {
  size?: 'sm' | 'md' | 'lg'
  label?: string
}

const sizeClasses = {
  sm: 'w-4 h-4 border-[1.5px]',
  md: 'w-6 h-6 border-2',
  lg: 'w-9 h-9 border-2',
}

const containerPadding = {
  sm: 'p-2',
  md: 'p-4',
  lg: 'p-6',
}

export function Spinner({ size = 'md', label = 'Cargando' }: SpinnerProps) {
  return (
    <div className={`flex flex-col items-center justify-center gap-2 ${containerPadding[size]}`}>
      <div
        className={`${sizeClasses[size]} border-border-subtle border-t-accent-secondary rounded-full animate-spin`}
        role="status"
        aria-label={label}
      />
    </div>
  )
}
