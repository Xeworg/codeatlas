import { useState } from 'react'
import {
  Search,
  GitBranch,
  BarChart3,
  MessageSquare,
  Settings,
  ChevronRight,
  ChevronDown,
  File,
  Folder,
  FolderOpen,
} from 'lucide-react'
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
        className={`flex items-center gap-1.5 px-2 py-1 text-xs cursor-pointer rounded-sm transition-colors ${
          isSelected
            ? 'bg-surface-active text-text-primary border-l-2 border-accent-primary'
            : 'text-text-secondary hover:bg-surface-hover'
        }`}
        style={{ paddingLeft: `${depth * 14 + 8}px` }}
        onClick={() => {
          if (isDir) setExpanded((e) => !e)
          else onSelect(node.id)
        }}
      >
        {/* Expand/collapse chevron or type icon */}
        {isDir ? (
          expanded ? (
            <ChevronDown size={12} className="text-text-muted flex-shrink-0" />
          ) : (
            <ChevronRight size={12} className="text-text-muted flex-shrink-0" />
          )
        ) : (
          <File size={12} className="text-text-muted flex-shrink-0" />
        )}
        {/* Folder or file icon */}
        {isDir ? (
          expanded ? (
            <FolderOpen size={12} className="text-text-muted flex-shrink-0" />
          ) : (
            <Folder size={12} className="text-text-muted flex-shrink-0" />
          )
        ) : null}
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

const RAIL_ICON_BUTTONS = [
  { icon: Search, label: 'Buscar' },
  { icon: GitBranch, label: 'Dependencias' },
  { icon: BarChart3, label: 'Estadísticas' },
  { icon: MessageSquare, label: 'Chat' },
  { icon: Settings, label: 'Configuración' },
] as const

export function Sidebar({ scanResult, selectedFileId, onSelectFile, onSearch }: SidebarProps) {
  const [search, setSearch] = useState('')
  const tree = scanResult ? buildTree(scanResult.files) : []

  const handleSearch = (value: string) => {
    setSearch(value)
    onSearch(value)
  }

  return (
    <aside className="flex bg-surface-elevated border-r border-border-subtle flex-shrink-0 overflow-hidden">
      {/* Icon rail */}
      <nav className="w-12 flex flex-col items-center py-2 gap-1 border-r border-border-subtle">
        {RAIL_ICON_BUTTONS.map(({ icon: Icon, label }) => (
          <button
            key={label}
            title={label}
            aria-label={label}
            className="w-9 h-9 flex items-center justify-center rounded-sm text-text-muted hover:text-text-secondary hover:bg-surface-hover transition-colors cursor-default"
          >
            <Icon size={18} />
          </button>
        ))}
      </nav>

      {/* File tree panel */}
      <div className="flex flex-col w-56">
        <div className="p-2 border-b border-border-subtle">
          <div className="relative flex items-center">
            <Search size={12} className="absolute left-2 text-text-muted pointer-events-none" />
            <input
              type="text"
              placeholder="Buscar archivos..."
              value={search}
              onChange={(e) => handleSearch(e.target.value)}
              className="w-full pl-7 pr-2 py-1 text-xs bg-surface-inset border border-border-subtle rounded-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-border-strong"
            />
          </div>
        </div>
        <div className="flex-1 overflow-y-auto py-1">
          {tree.length === 0 ? (
            <p className="text-xs text-text-muted p-3">Abrí un proyecto para ver los archivos</p>
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
      </div>
    </aside>
  )
}
