// ExportButton — dropdown for JSON/PNG export with fallback warning
// Part of PR4 (migrated to i18n in PR6)

import { useState, useRef, useEffect } from 'react'
import { useExport } from '../../hooks/useExport'
import { t } from '../../lib/i18n'

interface ExportButtonProps {
  /** The DOM element to capture for PNG export (typically the graph container). */
  graphElement?: HTMLElement | null
  /** Project ID used for filename generation. */
  projectId: string
  /** Additional CSS classes. */
  className?: string
}

export function ExportButton({ graphElement, projectId, className = '' }: ExportButtonProps) {
  const { status, error, fallbackWarning, exportJson, exportPng } = useExport()
  const [open, setOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  // Close menu on outside click
  useEffect(() => {
    if (!open) return
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClick)
    return () => document.removeEventListener('mousedown', handleClick)
  }, [open])

  const handleExportJson = () => {
    setOpen(false)
    exportJson(projectId)
  }

  const handleExportPng = () => {
    setOpen(false)
    exportPng(graphElement ?? null, projectId)
  }

  return (
    <div className={`relative inline-block ${className}`} ref={menuRef}>
      {/* Export button */}
      <button
        onClick={() => setOpen((v) => !v)}
        disabled={status === 'exporting'}
        className="flex items-center gap-1.5 px-3 py-1.5 rounded text-sm font-medium bg-gray-700 hover:bg-gray-600 disabled:opacity-50 text-gray-200 transition-colors"
        aria-label={t('export.title')}
        aria-expanded={open}
        aria-haspopup="menu"
      >
        {status === 'exporting' ? (
          <>
            <SpinnerIcon />
            {t('common.exporting')}
          </>
        ) : (
          <>
            <ExportIcon />
            {t('common.export')}
          </>
        )}
      </button>

      {/* Dropdown menu */}
      {open && (
        <div
          className="absolute right-0 mt-1 w-44 rounded shadow-lg bg-gray-800 border border-gray-700 z-50"
          role="menu"
        >
          <button
            onClick={handleExportJson}
            className="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-gray-700 flex items-center gap-2"
            role="menuitem"
          >
            <FileJsonIcon />
            {t('common.exportJson')}
          </button>
          <button
            onClick={handleExportPng}
            className="w-full text-left px-4 py-2 text-sm text-gray-200 hover:bg-gray-700 flex items-center gap-2"
            role="menuitem"
          >
            <ImageIcon />
            {t('common.exportPng')}
          </button>
        </div>
      )}

      {/* Fallback warning */}
      {fallbackWarning && (
        <div className="mt-2 px-3 py-2 rounded text-xs bg-yellow-900/40 border border-yellow-700 text-yellow-300">
          ⚠️ {fallbackWarning}
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="mt-2 px-3 py-2 rounded text-xs bg-red-900/40 border border-red-700 text-red-300">
          ✗ {error}
        </div>
      )}

      {/* Success toast */}
      {status === 'done' && (
        <div className="mt-2 px-3 py-2 rounded text-xs bg-green-900/40 border border-green-700 text-green-300">
          {t('export.jsonSuccess')}
        </div>
      )}
    </div>
  )
}

// Inline SVG icons to avoid external icon library dependency
function ExportIcon() {
  return (
    <svg
      className="w-4 h-4"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
      />
    </svg>
  )
}

function FileJsonIcon() {
  return (
    <svg
      className="w-4 h-4 text-blue-400"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
      />
    </svg>
  )
}

function ImageIcon() {
  return (
    <svg
      className="w-4 h-4 text-purple-400"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
      aria-hidden="true"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
      />
    </svg>
  )
}

function SpinnerIcon() {
  return (
    <svg className="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24" aria-hidden="true">
      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth={4} />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
      />
    </svg>
  )
}
