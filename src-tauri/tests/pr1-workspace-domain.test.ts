// RED test: Workspace domain should be queryable via Tauri commands.
// Expected to fail until PR1 implementation is complete.
import { describe, it, expect, beforeAll } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import type { Workspace, WorkspaceProject, Snapshot } from '../../src/lib/types-v3'

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
