# Spec Deltas: pre-wave-2-foundation — error-contract

> Delta spec for `openspec/specs/error-contract/spec.md`.
> Cambia el cumplimiento del contrato: la frontera IPC debe producir el envelope JSON estructurado, no strings sueltos.

## ADDED Requirements

### Requirement: IPC boundary emits structured IpcErrorPayload

The system MUST emit the canonical `IpcErrorPayload` JSON at the IPC boundary for every error returned by a Tauri command. The `to_ipc_error(e: AppError) -> String` helper in `src-tauri/src/lib.rs` (o módulo de nombre análogo) MUST be the single point of conversion from `AppError` to wire format. No command body may return a plain `e.to_string()`-style string as the IPC error.

#### Scenario: file-not-found from get_file_by_id

- **WHEN** `commands::get_node_details` invokes `repo.get_file_by_id(node_id)` and the file is absent
- **THEN** the command returns `Err(to_ipc_error(AppError::FileNotFound(node_id)))` whose serialized form is `{"code":"FILE_NOT_FOUND","message":"...","details":null,"trace_id":null}`
- **AND** the response is parseable by `toApiError` in `src/lib/tauri-api.ts` as a structured `ApiError`

#### Scenario: AI not configured from explain_node

- **WHEN** `AIService::explain_node_with_context` is called without an active provider
- **THEN** the command returns `Err(to_ipc_error(AppError::AIUnavailable))` with code `AI_UNAVAILABLE`
- **AND** the frontend renders a localized "AI no configurado" message keyed off `code`, not off string matching

#### Scenario: free-form string in command body is replaced

- **WHEN** a command body previously did `format!("File not found: {}", node_id)` or `"AI not configured".to_string()`
- **THEN** that string is replaced with the typed `AppError::FileNotFound(node_id)` / `AppError::AIUnavailable` variant before serialization
- **AND** the resulting `IpcErrorPayload` carries a stable `code`

#### Scenario: legacy string fallback in frontend is dead code

- **WHEN** all 37 command bodies route their error mapping through `to_ipc_error`
- **THEN** the legacy string-matching branch in `tauri-api.ts:96-125` is unreachable for normal flow
- **AND** tests assert that the structured `IpcErrorPayload` path is the one exercised

## MODIFIED Requirements

### Requirement: Atomic rollout of error contract

The change that updates the IPC boundary error format MUST land in a single commit. Backend helper, frontend parser/mapping, and affected test fixtures MUST update together. A change that only updates one side MUST be rejected in review. (Previously: atomicity was stated as "the backend serializer update and the frontend parser/mapping update MUST be merged together" at a conceptual level; the new contract is more concrete — it includes test fixtures and forbids the intermediate state where the backend emits structured payloads but the frontend still consumes legacy strings.)

#### Scenario: PR splits error boundary update

- **WHEN** a PR touches `src-tauri/src/commands.rs` to change error handling but does not update `src/lib/tauri-api.ts` and the affected test fixtures (`src/lib/__tests__/tauri-api.test.ts`, `src/hooks/__tests__/useAI-corrective.test.ts`, `src/services/__tests__/services-boundary.test.ts` if still present)
- **THEN** the PR is rejected in review with explicit reference to this requirement

#### Scenario: PR ships only the backend helper

- **WHEN** a PR adds the `to_ipc_error` helper and replaces `.map_err(|e| e.to_string())` but does not update the frontend parser or the test fixtures
- **THEN** the repo enters a broken intermediate state where the backend emits structured JSON and the frontend falls through to legacy string heuristics
- **AND** the change MUST be rejected before merge and re-shaped to be atomic
