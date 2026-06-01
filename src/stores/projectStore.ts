import { create } from 'zustand'
import type { ScanResult, ScanStatus } from '../lib/types'

interface ProjectState {
  // State
  projectId: string | null
  projectName: string | null
  rootPath: string | null
  scanResult: ScanResult | null
  status: ScanStatus
  progress: number
  error: string | null
  scanStartTime: number | null

  // Actions
  setProject: (id: string, name: string, rootPath: string) => void
  setScanResult: (result: ScanResult) => void
  setStatus: (status: ScanStatus) => void
  setProgress: (progress: number) => void
  setError: (error: string | null) => void
  clearProject: () => void
}

export const useProjectStore = create<ProjectState>((set) => ({
  projectId: null,
  projectName: null,
  rootPath: null,
  scanResult: null,
  status: 'idle',
  progress: 0,
  error: null,
  scanStartTime: null,

  setProject: (id, name, rootPath) =>
    set({ projectId: id, projectName: name, rootPath, scanStartTime: Date.now() }),

  setScanResult: (result) =>
    set({ scanResult: result, status: result.status, error: result.error ?? null }),

  setStatus: (status) => set({ status }),

  setProgress: (progress) => set({ progress }),

  setError: (error) => set({ error, status: 'error' }),

  clearProject: () =>
    set({
      projectId: null,
      projectName: null,
      rootPath: null,
      scanResult: null,
      status: 'idle',
      progress: 0,
      error: null,
      scanStartTime: null,
    }),
}))

// Selectors
export const useScanStatus = () => useProjectStore((s) => s.status)
export const useScanResult = () => useProjectStore((s) => s.scanResult)
export const useProjectId = () => useProjectStore((s) => s.projectId)
export const useProjectError = () => useProjectStore((s) => s.error)
