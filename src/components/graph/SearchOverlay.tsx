// SearchOverlay — search and highlight nodes in the graph
import { useCallback, useState } from 'react'
import { Search, X, ChevronUp, ChevronDown } from 'lucide-react'
import { useGraphStore } from '../../stores/graphStore'
import { useGraph } from '../../hooks/useGraph'
import { useGraphData } from '../../stores/graphStore'

export function SearchOverlay() {
  const searchQuery = useGraphStore((s) => s.searchQuery)
  const searchResults = useGraphStore((s) => s.searchResults)
  const selectNode = useGraphStore((s) => s.selectNode)
  const setSearchQuery = useGraphStore((s) => s.setSearchQuery)
  const setSearchResults = useGraphStore((s) => s.setSearchResults)
  const { search } = useGraph()
  const graphData = useGraphData()

  const [isOpen, setIsOpen] = useState(false)

  const handleInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const q = e.target.value
      setSearchQuery(q)
      search(q)
    },
    [search, setSearchQuery, setSearchResults]
  )

  const handleSelect = useCallback(
    (nodeId: string) => {
      selectNode(nodeId)
      setSearchQuery('')
      setSearchResults([])
      setIsOpen(false)
    },
    [selectNode, setSearchQuery, setSearchResults]
  )

  const handleClear = useCallback(() => {
    setSearchQuery('')
    setSearchResults([])
  }, [setSearchQuery, setSearchResults])

  if (!graphData) return null

  return (
    <div className="absolute top-4 right-4 z-10 w-72">
      <div className="bg-surface-base border border-border-subtle rounded-lg shadow-xl overflow-hidden">
        {/* Toggle */}
        <button
          onClick={() => setIsOpen((v) => !v)}
          className="w-full px-3 py-2 text-left text-sm text-text-secondary hover:bg-surface-hover flex items-center justify-between"
        >
          <span className="flex items-center gap-2">
            <Search size={14} className="text-text-muted" />
            Search nodes…
          </span>
          {isOpen ? (
            <ChevronUp size={14} className="text-text-muted" />
          ) : (
            <ChevronDown size={14} className="text-text-muted" />
          )}
        </button>

        {/* Input + results */}
        {isOpen && (
          <div className="border-t border-border-subtle">
            <div className="p-2">
              <input
                type="text"
                value={searchQuery}
                onChange={handleInput}
                placeholder="Filter by name or path…"
                className="w-full px-3 py-1.5 bg-surface-inset border border-border-subtle rounded text-sm text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent-primary"
                autoFocus
              />
              {searchQuery && (
                <button
                  onClick={handleClear}
                  className="mt-1 text-xs text-text-muted hover:text-text-secondary flex items-center gap-1"
                >
                  <X size={10} />
                  Clear
                </button>
              )}
            </div>

            {/* Results */}
            {searchResults.length > 0 && (
              <ul className="max-h-60 overflow-y-auto border-t border-border-subtle">
                {searchResults.slice(0, 10).map((node) => (
                  <li key={node.id}>
                    <button
                      onClick={() => handleSelect(node.id)}
                      className="w-full px-3 py-2 text-left hover:bg-surface-hover border-b border-border-subtle last:border-0"
                    >
                      <div className="text-xs font-medium text-text-primary truncate">
                        {node.label}
                      </div>
                      <div className="text-xs text-text-muted truncate">{node.path}</div>
                    </button>
                  </li>
                ))}
              </ul>
            )}

            {searchQuery && searchResults.length === 0 && (
              <div className="px-3 py-2 text-xs text-text-muted">No results</div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
