// tauri-api-bridge.test.ts
// ─────────────────────────────────────────────────────────────────────────────
// Documents the tauri-api bridge layer — how frontend code connects to Tauri.
//
// This file was migrated from src/services/__tests__/services-boundary.test.ts
// during PR-C B.13.1. The original "T19 RED" framing is stale: it was written
// during a TDD RED phase for a service layer that was never fully implemented.
// The GREEN phase that would have replaced these tests never materialized because
// the service layer is being deleted entirely (B.13.2/B.13.3).
//
// What this file DOES test (bridge-level):
//   1. smoke — tauriApi module exports are callable via the @tauri-apps/api/core bridge
//   2. parser — toApiError correctly strips the Tauri "Error: " prefix and parses
//      Tauri-wrapped JSON errors (the kind B.12's to_ipc_error produces)
//
// What this file does NOT test (covered elsewhere):
//   - Parser unit tests → src/lib/__tests__/tauri-api.test.ts (38 tests)
//   - Static guard verification → scripts/ci/check-architecture.mjs (B.13.3)
//   - Service-layer contracts → src/services/*.ts (being deleted in B.13.2)
// ─────────────────────────────────────────────────────────────────────────────

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { toApiError } from '../tauri-api'

beforeEach(() => {
  vi.clearAllMocks()
})

// ─────────────────────────────────────────────────────────────────────────────
// Bridge smoke — verifies tauriApi module forwards calls to @tauri-apps/api/core
// ─────────────────────────────────────────────────────────────────────────────

describe('tauri-api bridge', () => {
  describe('smoke — tauriApi module is callable via @tauri-apps/api/core', () => {
    it('scanProject returns a ScanResult without throwing', async () => {
      // Mock @tauri-apps/api/core at the module level so scanProject resolves
      // without actually calling Tauri. The smoke test only verifies the bridge
      // is wired: import resolves → function is callable → returns expected shape.
      vi.mock('@tauri-apps/api/core', () => ({
        invoke: vi.fn().mockResolvedValue({
          projectId: 'proj-smoke',
          projectName: 'Smoke',
          rootPath: '/tmp/smoke',
          filesCount: 1,
          symbolsCount: 2,
          importsCount: 0,
          files: [],
          scanDurationMs: 10,
          status: 'ready' as const,
        }),
      }))

      // Re-import after mocking so the mock is active
      vi.resetModules()
      const { scanProject } = await import('../tauri-api')

      const result = await scanProject('/tmp/smoke')

      expect(result.projectId).toBe('proj-smoke')
      expect(result.status).toBe('ready')
    })
  })

  // ─────────────────────────────────────────────────────────────────────────
  // Parser bridge — verifies toApiError handles Tauri-prefixed JSON errors
  //
  // When the backend returns an error (e.g. from to_ipc_error in B.12), Tauri
  // wraps it as: Error: {"code":"FILE_NOT_FOUND","message":"x.txt"}
  // The "Error: " prefix is added by Tauri; the JSON is produced by the backend.
  // toApiError strips the prefix and parses the JSON to produce a typed ApiError.
  // ─────────────────────────────────────────────────────────────────────────

  describe('parser — toApiError bridges Tauri-wrapped JSON errors', () => {
    it('strips Tauri prefix and parses structured JSON error from B.12 backend', () => {
      // Simulate what Tauri returns after B.12's to_ipc_error wraps a backend error:
      // Error: {"code":"FILE_NOT_FOUND","message":"x.txt"}
      const tauriError = new Error('Error: {"code":"FILE_NOT_FOUND","message":"x.txt"}')

      const result = toApiError(tauriError)

      // code should map FILE_NOT_FOUND → PATH_NOT_FOUND per BACKEND_TO_FRONTEND_CODE
      expect(result.code).toBe('PATH_NOT_FOUND')
      expect(result.message).toBe('x.txt')
    })

    it('returns fallback INTERNAL code when Tauri error has no structured JSON', () => {
      const plainError = new Error('Error: something went wrong')
      const result = toApiError(plainError)

      expect(result.code).toBe('INTERNAL')
      expect(result.message).toBe('something went wrong')
    })
  })
})