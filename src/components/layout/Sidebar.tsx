import { useState } from 'react'
import type { ScanResult } from '../../lib/types'

interface SidebarProps {
  scanResult: ScanResult | null
  selectedFileId: string | null
  onSelectFile: (fileId: string) => void
  onSearch: (query: string) => void
}

interface FileTreeNode {
  name: string
  path: string
  id: string
  children?: FileTreeNode[]
}

function buildTree(files: ScanResult['files']): FileTreeNode[] {
  const root: FileTreeNode[] = []
  const dirs: Record<string, FileTreeNode> = {}

  for (const file of files) {
    const parts = file.path.split('/')
    let current = root
    let currentPath = ''

    for (let i = 0; i < parts.length - 1; i++) {
      currentPath += parts[i] + '/'
      if (!dirs[currentPath]) {
        const node: FileTreeNode = { name: parts[i], path: currentPath, id: currentPath }
        dirs[currentPath] = node
        current.push(node)
      }
      current = dirs[currentPath].children ?? (dirs[currentPath].children = [])
    }

    current.push({ name: parts[parts.length - 1], path: file.path, id: file.id })
  }

  return root
}

function TreeNode({
  node,
  depth,
  selectedId,
  onSelect,
}: {
  node: FileTreeNode
  depth: number
  selectedId: string | null
  onSelect: (id: string) => void
}) {
  const [expanded, setExpanded] = useState(depth < 2)
  const isDir = !!node.children
  const isSelected = node.id === selectedId

  return (
    <div>
      <div
        className={`flex items-center gap-1 px-2 py-0.5 text-xs cursor-pointer rounded hover:bg-gray-700 ${
          isSelected ? 'bg-blue-900 text-blue-300' : 'text-gray-300'
        }`}
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
        onClick={() => {
          if (isDir) setExpanded((e) => !e)
          else onSelect(node.id)
        }}
      >
        <span className="text-gray-500">{isDir ? (expanded ? '▼' : '▶') : '·'}</span>
        <span className="truncate">{node.name}</span>
      </div>
      {isDir &&
        expanded &&
        node.children?.map((child) => (
          <TreeNode
            key={child.id}
            node={child}
            depth={depth + 1}
            selectedId={selectedId}
            onSelect={onSelect}
          />
        ))}
    </div>
  )
}

export function Sidebar({ scanResult, selectedFileId, onSelectFile, onSearch }: SidebarProps) {
  const [search, setSearch] = useState('')
  const tree = scanResult ? buildTree(scanResult.files) : []

  const handleSearch = (value: string) => {
    setSearch(value)
    onSearch(value)
  }

  return (
    <aside className="w-56 bg-gray-850 border-r border-gray-700 flex flex-col flex-shrink-0">
      <div className="p-2 border-b border-gray-700">
        <input
          type="text"
          placeholder="Buscar archivos..."
          value={search}
          onChange={(e) => handleSearch(e.target.value)}
          className="w-full px-2 py-1 text-xs bg-gray-800 border border-gray-600 rounded text-gray-200 placeholder-gray-500 focus:outline-none focus:border-blue-500"
        />
      </div>
      <div className="flex-1 overflow-y-auto py-1">
        {tree.length === 0 ? (
          <p className="text-xs text-gray-500 p-3">Abrí un proyecto para ver los archivos</p>
        ) : (
          tree.map((node) => (
            <TreeNode
              key={node.id}
              node={node}
              depth={0}
              selectedId={selectedFileId}
              onSelect={onSelectFile}
            />
          ))
        )}
      </div>
    </aside>
  )
}
