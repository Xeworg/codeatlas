// Simple tab switcher for panel sections
// Part of PR5b (AI UI)

interface Tab {
  id: string
  label: string
  icon?: string
}

interface TabSwitcherProps {
  tabs: Tab[]
  activeTab: string
  onChange: (id: string) => void
}

export function TabSwitcher({ tabs, activeTab, onChange }: TabSwitcherProps) {
  return (
    <div className="flex border-b border-gray-200 bg-white">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onChange(tab.id)}
          className={`flex items-center gap-1.5 px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px ${
            activeTab === tab.id
              ? 'border-blue-500 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
          }`}
        >
          {tab.icon && <span className="text-base">{tab.icon}</span>}
          <span>{tab.label}</span>
        </button>
      ))}
    </div>
  )
}
