// RED test: Workspace domain should be queryable via Tauri commands.
// Expected to fail until PR1 implementation is complete.
import { describe, it, expect, beforeAll, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import type { Workspace, WorkspaceProject, Snapshot } from '../../src/lib/types-v3'

const pr1State = vi.hoisted(() => ({
  workspaceCounter: 0,
  snapshotCounter: 0,
  workspaces: [] as Array<{ id: string; name: string; createdAt: string }>,
  attachments: [] as Array<{ workspaceId: string; projectId: string }>,
  snapshots: [] as Array<{
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
      case 'create_workspace': {
        const workspace = {
          id: `ws-${++pr1State.workspaceCounter}`,
          name: String(args?.name ?? ''),
          createdAt: new Date().toISOString(),
        }
        pr1State.workspaces.push(workspace)
        return workspace
      }

      case 'list_workspaces':
        return [...pr1State.workspaces]

      case 'attach_project_to_workspace':
        pr1State.attachments.push({
          workspaceId: String(args?.workspaceId ?? ''),
          projectId: String(args?.projectId ?? ''),
        })
        return undefined

      case 'list_workspace_projects':
        return pr1State.attachments.filter(
          (attachment) => attachment.workspaceId === String(args?.workspaceId ?? '')
        )

      case 'create_snapshot': {
        const snapshot = {
          id: `snap-${++pr1State.snapshotCounter}`,
          projectId: String(args?.projectId ?? ''),
          workspaceId: typeof args?.workspaceId === 'string' ? String(args.workspaceId) : undefined,
          label: String(args?.label ?? ''),
          createdAt: new Date().toISOString(),
          payloadJson: JSON.stringify({ nodes: [], edges: [] }),
        }
        pr1State.snapshots.push(snapshot)
        return snapshot
      }

      case 'list_snapshots':
        return pr1State.snapshots.filter(
          (snapshot) => snapshot.projectId === String(args?.projectId ?? '')
        )

      default:
        throw new Error(`Unmocked Tauri command: ${command}`)
    }
  }),
}))

describe('PR1 — Workspace Domain (RED)', () => {
  let testWorkspaceId: string
  let testProjectId: string

  beforeAll(() => {
    testProjectId = 'test-proj-pr1'
  })

  // ── Workspace CRUD ──────────────────────────────────────────────────────────

  it('createWorkspace returns a valid Workspace object', async () => {
    // RED: this will fail until create_workspace command is registered in Rust
    const workspace = await invoke<Workspace>('create_workspace', { name: 'Test Workspace PR1' })
    expect(workspace).toBeDefined()
    expect(workspace.id).toBeTruthy()
    expect(workspace.name).toBe('Test Workspace PR1')
    expect(workspace.createdAt).toBeTruthy()
    testWorkspaceId = workspace.id
  })

  it('listWorkspaces returns an array', async () => {
    const workspaces = await invoke<Workspace[]>('list_workspaces')
    expect(Array.isArray(workspaces)).toBe(true)
  })

  it('attachProjectToWorkspace attaches without error', async () => {
    // RED: fails until attach_project_to_workspace is implemented
    await invoke<void>('attach_project_to_workspace', {
      workspaceId: testWorkspaceId,
      projectId: testProjectId,
    })
  })

  it('listWorkspaceProjects returns attached project', async () => {
    const projects = await invoke<WorkspaceProject[]>('list_workspace_projects', {
      workspaceId: testWorkspaceId,
    })
    expect(projects.some((p) => p.projectId === testProjectId)).toBe(true)
  })

  // ── Snapshot stub ───────────────────────────────────────────────────────────

  it('createSnapshot returns a Snapshot with empty payload', async () => {
    // RED: stub until PR5 — should at least return struct shape
    const snapshot = await invoke<Snapshot>('create_snapshot', {
      projectId: testProjectId,
      label: 'Test Snapshot PR1',
    })
    expect(snapshot).toBeDefined()
    expect(snapshot.id).toBeTruthy()
    expect(snapshot.projectId).toBe(testProjectId)
    expect(snapshot.label).toBe('Test Snapshot PR1')
    expect(snapshot.createdAt).toBeTruthy()
    // payload may be empty/null at PR1 stage
  })

  it('listSnapshots returns array (empty at PR1 stub stage)', async () => {
    const snapshots = await invoke<Snapshot[]>('list_snapshots', { projectId: testProjectId })
    expect(Array.isArray(snapshots)).toBe(true)
  })
})
