// ===========================================================================
// useSnapshotStore — Zustand store for snapshot state (PR5)
// Aligned with openspec/changes/v3-collaboration-platform/design.md
// Part of PR-8 (Frontend services/hooks)
// ===========================================================================
import { create } from 'zustand'
import { createSnapshot, listSnapshots, getSnapshot } from '../services/snapshotService'
import type { Snapshot, SnapshotPayload } from '../lib/types-v3'

interface SnapshotState {
  snapshots: Snapshot[]
  activeSnapshotId: string | null
  isLoading: boolean
  error: string | null

  // Actions
  createSnapshot: (projectId: string, label: string, workspaceId?: string) => Promise<void>
  listSnapshots: (projectId: string, workspaceId?: string) => Promise<void>
  loadSnapshot: (snapshotId: string) => Promise<SnapshotPayload | null>
  clearActiveSnapshot: () => void
  clearError: () => void
}

export const useSnapshotStore = create<SnapshotState>((set, get) => ({
  snapshots: [],
  activeSnapshotId: null,
  isLoading: false,
  error: null,

  createSnapshot: async (projectId: string, label: string, workspaceId?: string) => {
    set({ isLoading: true, error: null })
    try {
      const snap = await createSnapshot(projectId, label, workspaceId)
      const current = get().snapshots
      set({ snapshots: [snap, ...current], isLoading: false })
    } catch (e) {
      set({ error: String(e), isLoading: false })
    }
  },

  listSnapshots: async (projectId: string, workspaceId?: string) => {
    set({ isLoading: true, error: null })
    try {
      const snaps = await listSnapshots(projectId, workspaceId)
      set({ snapshots: snaps, isLoading: false })
    } catch (e) {
      set({ error: String(e), isLoading: false })
    }
  },

  loadSnapshot: async (snapshotId: string) => {
    set({ isLoading: true, error: null })
    try {
      const snap = await getSnapshot(snapshotId)
      if (!snap) {
        set({ error: 'Snapshot not found', isLoading: false })
        return null
      }
      set({ activeSnapshotId: snapshotId, isLoading: false })
      const payload: SnapshotPayload | null = snap.payloadJson
        ? (JSON.parse(snap.payloadJson) as SnapshotPayload)
        : null
      return payload
    } catch (e) {
      set({ error: String(e), isLoading: false })
      return null
    }
  },

  clearActiveSnapshot: () => {
    set({ activeSnapshotId: null })
  },

  clearError: () => {
    set({ error: null })
  },
}))
