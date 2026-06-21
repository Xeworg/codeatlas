# Error Contract — Wave 2 Delta (C3b)

> **Change**: `wave-2-hexagonal-completion`
> **Sub-scope**: `c3b/`
> **Status**: Ready for sdd-apply
> **Locked decisions applied** (recovery, 2026-06-12):
>
> - Standardize on existing `AppError` / `IpcErrorPayload` / `ErrorCode` / `ApiError`. Do NOT introduce new `AppError` variants in this slice.
> - "AI not configured" maps to existing `AppError::AIUnavailable("AI not configured".to_string())` (code `AI_UNAVAILABLE`).
> - "File not found" maps to existing `AppError::FileNotFound(path)` (code `FILE_NOT_FOUND`).
> - Transitional frontend helper path: `src/lib/errors.ts`. Long-term direction: presentation-owned user-facing error messaging.
> - Spanish i18n strings live in `src/locales/es/errors.ts` (TypeScript module, not added to `es.json`).
> - Tauri commands MUST route errors through `to_ipc_error`; direct `.to_string()` errors are forbidden in `src-tauri/src/commands.rs`.

## ADDED Requirements

### Requirement: Tauri commands route every error through `to_ipc_error`

The presentation-layer command surface in `src-tauri/src/commands.rs` MUST NOT construct `String` errors directly. Every command return path MUST go through `to_ipc_error` so the structured `IpcErrorPayload` (code + message + details) survives the IPC boundary.

#### Scenario: AI-not-configured uses AppError::AIUnavailable

- GIVEN the `explain_node` or `chat` command runs while `state.ai_config` is `None`
- WHEN the command converts the missing config to an error
- THEN the command MUST construct `AppError::AIUnavailable("AI not configured".to_string())`
- AND it MUST return that error via `to_ipc_error`
- AND the wire payload MUST carry `code = "AI_UNAVAILABLE"` and `details.reason = "AI not configured"`

#### Scenario: Missing file metadata uses AppError::FileNotFound

- GIVEN the `explain_node` command runs with a `node_id` that has no row in `scan_repo.get_file_by_id`
- WHEN the command converts the missing file to an error
- THEN the command MUST construct `AppError::FileNotFound(node_id)`
- AND post-C3a this construction site lives in `engine::ai::service::AIServicePort::explain_node`, not in the Tauri shim
- AND it MUST return that error via `to_ipc_error`
- AND the wire payload MUST carry `code = "FILE_NOT_FOUND"` and `details.path = node_id`

#### Scenario: No string-literal error remains in commands.rs

- GIVEN the post-C3b working tree
- WHEN a reviewer runs `rg '\.to_string\(\)|format!\("' src-tauri/src/commands.rs`
- THEN every match MUST be either a non-error construction (status string, UUID stringification) or a `format!` inside an `AppError` variant constructor
- AND no match MUST be a top-level `?` operator or `ok_or_else` returning a raw `String` error

### Requirement: Frontend i18n boundary lives in `src/lib/errors.ts`

User-facing error message translation MUST live in a single dedicated module (`src/lib/errors.ts`) so `tauri-api.ts` stays an IPC transport. The legacy inlined `toUserMessage` / `getErrorMessage` in `tauri-api.ts` MUST be removed and re-exported from `errors.ts` for backward compatibility.

#### Scenario: `toUserMessage` and `getErrorMessage` resolve from `src/lib/errors.ts`

- GIVEN any consumer imports `toUserMessage` or `getErrorMessage`
- WHEN the import resolves
- THEN the function MUST be defined in `src/lib/errors.ts`
- AND `src/lib/tauri-api.ts` MUST re-export them for backward compatibility
- AND the runtime behavior MUST be byte-identical to the pre-C3b implementation

#### Scenario: Spanish messages are co-located in `src/locales/es/errors.ts`

- GIVEN the `toUserMessage` function for code `PATH_NOT_FOUND`
- WHEN the function returns the user-facing string
- THEN the string MUST originate from the `src/locales/es/errors.ts` mapping
- AND the mapping MUST cover at least every code in the frontend `ErrorCode` union

### Requirement: Backend-to-frontend error roundtrip is exercised end-to-end

The contract MUST have an integration test that simulates a full backend `AppError` → IPC payload → frontend `toApiError` → `toUserMessage` path so a regression in any layer is caught.

#### Scenario: TE3 roundtrip test covers AI_UNAVAILABLE and FILE_NOT_FOUND

- GIVEN the integration test `engine/tests/error_roundtrip_test.rs` (or frontend equivalent)
- WHEN the test serializes an `AppError` via `to_ipc_error`
- AND the frontend `toApiError` parses the resulting JSON
- AND `toUserMessage` resolves the parsed `ApiError`
- THEN the resolved Spanish string MUST match the expected localized message for that code
- AND the test MUST cover both `AI_UNAVAILABLE` (→ `UNREACHABLE` → Spanish) and `FILE_NOT_FOUND` (→ `PATH_NOT_FOUND` → Spanish) at minimum

## Cross-references

- Parent proposal: `openspec/changes/wave-2-hexagonal-completion/proposal.md` (C3b §In Scope)
- Parent design: `openspec/changes/wave-2-hexagonal-completion/design.md` (T4, F2, TE3)
- C3b exploration: `openspec/changes/wave-2-hexagonal-completion/c3b/exploration.md` §10
- C3b tasks: `openspec/changes/wave-2-hexagonal-completion/c3b/tasks.md`
- Canonical spec: `openspec/specs/error-contract/spec.md` (merged at archive)
- Source observation: Engram #675 (error-contract modified-delta)
