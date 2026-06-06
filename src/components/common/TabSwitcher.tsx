// TabSwitcher — segmented control for panel sections
// Slice 4 (milestone 2): icon is now a ReactNode so callers can pass
// Lucide components or any other icon node. Restyled to the dark
// reference palette: recessed inset track, subtle separators, violet
// accent underline on the active tab, and muted inactive labels.
import type { ReactNode } from 'react'

export interface Tab {
  id: string
  label: string
  icon?: ReactNode
}

interface TabSwitcherProps {
  tabs: Tab[]
  activeTab: string
  onChange: (id: string) => void
}

export function TabSwitcher({ tabs, activeTab, onChange }: TabSwitcherProps) {
  return (
    <div
      role="tablist"
      className="flex bg-surface-inset border-b border-border-subtle"
    >
      {tabs.map((tab) => {
        const isActive = activeTab === tab.id
        return (
          <button
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            onClick={() => onChange(tab.id)}
            className={`group relative flex items-center gap-2 px-4 py-2 text-xs font-medium transition-colors ${
              isActive
                ? 'text-text-primary'
                : 'text-text-muted hover:text-text-secondary'
            }`}
          >
            {tab.icon && (
              <span
                aria-hidden
                className={`inline-flex items-center justify-center transition-colors ${
                  isActive ? 'text-accent-secondary' : 'text-text-muted group-hover:text-text-secondary'
                }`}
              >
                {tab.icon}
              </span>
            )}
            <span>{tab.label}</span>
            {/* Active underline — violet accent matches the reference's tab indicator */}
            {isActive && (
              <span
                aria-hidden
                className="pointer-events-none absolute inset-x-0 -bottom-px h-0.5 bg-accent-secondary"
              />
            )}
          </button>
        )
      })}
    </div>
  )
}
