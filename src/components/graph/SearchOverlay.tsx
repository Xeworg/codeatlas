// SearchOverlay — search and highlight nodes in the graph
import { useCallback, useState } from 'react'
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
      <div className="bg-slate-900 border border-slate-700 rounded-lg shadow-xl overflow-hidden">
        {/* Toggle */}
        <button
          onClick={() => setIsOpen((v) => !v)}
          className="w-full px-3 py-2 text-left text-sm text-slate-300 hover:bg-slate-800 flex items-center justify-between"
        >
          <span>🔍 Search nodes…</span>
          {isOpen ? <span>▲</span> : <span>▼</span>}
        </button>

        {/* Input + results */}
        {isOpen && (
          <div className="border-t border-slate-700">
            <div className="p-2">
              <input
                type="text"
                value={searchQuery}
                onChange={handleInput}
                placeholder="Filter by name or path…"
                className="w-full px-3 py-1.5 bg-slate-800 border border-slate-600 rounded text-sm text-slate-200 placeholder-slate-500 focus:outline-none focus:border-blue-500"
                autoFocus
              />
              {searchQuery && (
                <button
                  onClick={handleClear}
                  className="mt-1 text-xs text-slate-500 hover:text-slate-300"
                >
                  Clear
                </button>
              )}
            </div>

            {/* Results */}
            {searchResults.length > 0 && (
              <ul className="max-h-60 overflow-y-auto border-t border-slate-700">
                {searchResults.slice(0, 10).map((node) => (
                  <li key={node.id}>
                    <button
                      onClick={() => handleSelect(node.id)}
                      className="w-full px-3 py-2 text-left hover:bg-slate-800 border-b border-slate-800 last:border-0"
                    >
                      <div className="text-xs font-medium text-slate-200 truncate">
                        {node.label}
                      </div>
                      <div className="text-xs text-slate-500 truncate">{node.path}</div>
                    </button>
                  </li>
                ))}
              </ul>
            )}

            {searchQuery && searchResults.length === 0 && (
              <div className="px-3 py-2 text-xs text-slate-500">No results</div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
