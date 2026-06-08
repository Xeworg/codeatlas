// snapshotService — domain-oriented wrapper for snapshot operations
// Part of PR-8 (Frontend services/hooks)
// Bridges the gap between stores and tauri-api for snapshot management.

import {
  createSnapshot as _createSnapshot,
  listSnapshots as _listSnapshots,
  getSnapshot as _getSnapshot,
} from '../lib/tauri-api'
import type { Snapshot } from '../lib/types-v3'

// ─── Snapshots ─────────────────────────────────────────────────────────────

/**
 * Create a new named snapshot of the current project state.
 */
export async function createSnapshot(
  projectId: string,
  label: string,
  workspaceId?: string
): Promise<Snapshot> {
  return _createSnapshot(projectId, label, workspaceId)
}

/**
 * List all snapshots for a project, optionally filtered by workspace.
 */
export async function listSnapshots(projectId: string, workspaceId?: string): Promise<Snapshot[]> {
  return _listSnapshots(projectId, workspaceId)
}

/**
 * Load a specific snapshot by ID.
 * Returns null if the snapshot does not exist.
 */
export async function getSnapshot(snapshotId: string): Promise<Snapshot | null> {
  return _getSnapshot(snapshotId)
}
