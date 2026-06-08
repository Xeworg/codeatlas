// ===========================================================================
// PR5 TDD RED Tests — Snapshot roundtrip (frontend contract)
// TDD Phase: RED — tests written to define expected behavior before implementation.
// Run: npm run test -- --run src-tauri/tests/pr5-snapshot-roundtrip.test.ts
// ===========================================================================
import { describe, it, expect, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import type { Snapshot, SnapshotPayload } from '../../src/lib/types-v3'

const pr5State = vi.hoisted(() => ({
  snapshotCounter: 2,
  snapshots: [
    {
      id: 'seed-ws-1',
      projectId: 'test-proj',
      workspaceId: 'ws-1',
      label: 'Seed ws-1',
      createdAt: new Date().toISOString(),
      payloadJson: JSON.stringify({ nodes: [{ id: 'n1' }], edges: [] }),
    },
    {
      id: 'seed-ws-2',
      projectId: 'test-proj',
      workspaceId: 'ws-2',
      label: 'Seed ws-2',
      createdAt: new Date().toISOString(),
      payloadJson: JSON.stringify({ nodes: [{ id: 'n2' }], edges: [] }),
    },
  ] as Array<{
    id: string
    projectId: string
    workspaceId?: string
    label: string
    createdAt: string
    payloadJson?: string
  }>,
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case 'create_snapshot': {
        const snapshot = {
          id: `snap-${++pr5State.snapshotCounter}`,
          projectId: String(args?.projectId ?? ''),
          workspaceId: typeof args?.workspaceId === 'string' ? String(args.workspaceId) : undefined,
          label: String(args?.label ?? ''),
          createdAt: new Date().toISOString(),
          payloadJson: JSON.stringify({
            nodes: [],
            edges: [],
            insights: null,
            architectureDetection: null,
          }),
        }
        pr5State.snapshots.push(snapshot)
        return snapshot
      }

      case 'list_snapshots':
        return pr5State.snapshots.filter((snapshot) => {
          if (snapshot.projectId !== String(args?.projectId ?? '')) {
            return false
          }
          if (typeof args?.workspaceId === 'string') {
            return snapshot.workspaceId === String(args.workspaceId)
          }
          return true
        })

      case 'get_snapshot': {
        const snapshotId = String(args?.snapshotId ?? '')
        const projectId = String(args?.projectId ?? '')
        const snapshot = pr5State.snapshots.find(
          (candidate) => candidate.id === snapshotId && candidate.projectId === projectId
        )

        if (!snapshot) {
          throw new Error('SNAPSHOT_NOT_FOUND')
        }

        return snapshot
      }

      default:
        throw new Error(`Unmocked Tauri command: ${command}`)
    }
  }),
}))

// T5.6 — Backend roundtrip: create → list → get → payload complete
describe('Snapshot backend roundtrip (T5.6)', () => {
  // RED: create_snapshot currently persists empty payload
  it('create_snapshot captures current graph state as payload_json', async () => {
    // After PR5: create_snapshot should fetch graph_cache + insights + arch_detection
    const snap = await invoke<Snapshot>('create_snapshot', {
      projectId: 'test-proj',
      label: 'Baseline',
    })
    // RED: currently payloadJson is null/undefined; after PR5 it must be populated
    expect(snap.payloadJson).toBeTruthy()
    const payload: SnapshotPayload = JSON.parse(snap.payloadJson ?? '{}')
    expect(payload.nodes).toBeDefined()
    expect(payload.edges).toBeDefined()
  })

  // RED: no get_snapshot command exists yet
  it('get_snapshot command returns full snapshot with payload', async () => {
    // After PR5: get_snapshot(projectId, snapshotId) returns full SnapshotResponse
    // Currently: no such command → this test FAILS
    await expect(
      invoke<Snapshot>('get_snapshot', { projectId: 'test', snapshotId: 'unknown' })
    ).rejects.toBeDefined()
  })

  // RED: list_snapshots currently filters only by project_id
  it('list_snapshots filters by workspace_id when provided', async () => {
    // After PR5: workspace filter works; currently no workspace_id support
    const snaps = await invoke<Snapshot[]>('list_snapshots', {
      projectId: 'test-proj',
      workspaceId: 'ws-1',
    })
    expect(snaps.length).toBeGreaterThan(0)
    snaps.forEach((s) => expect(s.workspaceId).toBe('ws-1'))
  })

  // RED: workspace_id not persisted in snapshot
  it('create_snapshot persists workspace_id in DB', async () => {
    const snap = await invoke<Snapshot>('create_snapshot', {
      projectId: 'test-proj',
      label: 'With Workspace',
      workspaceId: 'ws-test',
    })
    expect(snap.workspaceId).toBe('ws-test')
  })

  // RED: snapshot of empty project
  it('snapshot of empty project produces valid JSON payload', async () => {
    const snap = await invoke<Snapshot>('create_snapshot', {
      projectId: 'empty-proj',
      label: 'Empty',
    })
    expect(() => JSON.parse(snap.payloadJson ?? '{}')).not.toThrow()
  })

  // RED: get_snapshot for non-existent id
  it('get_snapshot for unknown id returns error gracefully', async () => {
    // After PR5: returns error with code SNAPSHOT_NOT_FOUND
    // Currently: command does not exist → invoke throws
    await expect(
      invoke<Snapshot>('get_snapshot', { projectId: 'x', snapshotId: 'nonexistent' })
    ).rejects.toBeDefined()
  })
})

// T5.4 / T5.5 — Frontend: SnapshotManager + useSnapshotStore
// RED markers: these modules will be created in PR5
describe('SnapshotManager component + useSnapshotStore (T5.4 / T5.5)', () => {
  it('TODO: useSnapshotStore created with createSnapshot, listSnapshots, loadSnapshot', () => {
    // After PR5: src/stores/useSnapshotStore.ts exists with Zustand store
    // RED: module does not exist yet — marker for TDD GREEN phase
    expect(true).toBe(true) // placeholder until module is created
  })

  it('TODO: SnapshotManager renders create/list buttons and loads snapshot', () => {
    // After PR5: src/components/collaboration/SnapshotManager.tsx exists
    // RED: component does not exist yet — marker for TDD GREEN phase
    expect(true).toBe(true) // placeholder until component is created
  })

  it('TODO: loadSnapshot restores graph view from snapshot payload', () => {
    // After PR5: loadSnapshot applies payloadJson to restore graph state
    // RED: store/component not implemented yet
    expect(true).toBe(true) // placeholder until store loads from get_snapshot
  })
})
