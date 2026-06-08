# Error Contract Specification

## Purpose

Define the stable backend-to-frontend error contract for CodeAtlas during the first hexagonal migration wave.

## Requirements

### Requirement: IPC-safe structured error payload

The backend MUST emit a structured error payload over the Tauri IPC error channel while respecting the runtime reality that the channel is string-oriented.

#### Scenario: Structured payload is transported as JSON string

- GIVEN any `AppError` crossing the Tauri command boundary
- WHEN the backend serializes that error for IPC
- THEN the payload MUST represent a structured object with `code`, `message`, and optional `details`
- AND the transmitted value MUST be a JSON string that the frontend can parse from the thrown error message

#### Scenario: Human-readable message is preserved

- GIVEN any serialized backend error
- WHEN the frontend or logs inspect the payload
- THEN the payload MUST preserve a human-readable `message`
- AND the migration MUST NOT reduce debuggability compared with the current plain-string contract

### Requirement: Stable backend error code catalog

The backend MUST maintain a stable catalog of uppercase snake_case error codes for the existing `AppError` variants.

#### Scenario: Every relevant AppError variant has a stable code

- GIVEN the backend `AppError` enum
- WHEN inspecting the serialization mapping
- THEN the mapping MUST cover at least:
  - `PROJECT_NOT_FOUND`
  - `FILE_NOT_FOUND`
  - `SCAN_TIMEOUT`
  - `DATABASE`
  - `AI_UNAVAILABLE`
  - `AI_RATE_LIMITED`
  - `AI_TOKEN_LIMIT`
  - `INVALID_API_KEY`
  - `ACCESS_DENIED`
  - `INTERNAL`

#### Scenario: Structured details remain structured

- GIVEN an error variant with contextual payload
- WHEN it is serialized
- THEN the `details` field MUST remain machine-readable data
- AND the frontend-facing `ApiError.details` contract MUST stay compatible with `Record<string, unknown> | undefined`

### Requirement: Frontend parses structured errors first, legacy second

The frontend MUST prefer the structured contract and retain a temporary fallback for legacy plain-string backend errors.

#### Scenario: toApiError parses structured JSON payload

- GIVEN a thrown Tauri error whose `message` contains valid JSON with `code` and `message`
- WHEN `toApiError` processes it
- THEN the function MUST parse the JSON first
- AND it MUST produce a typed `ApiError`

#### Scenario: Legacy fallback remains available during rollout

- GIVEN a thrown Tauri error whose `message` is not valid structured JSON
- WHEN `toApiError` processes it
- THEN the function MUST fall back to the existing legacy string heuristics
- AND the fallback MUST remain only as a migration aid, not as the primary contract

### Requirement: Explicit backend-to-frontend code mapping

The frontend error union does not need to mirror backend codes 1:1, but the mapping MUST be explicit and complete.

#### Scenario: Existing frontend ErrorCode union is fully covered

- GIVEN the current frontend `ErrorCode` union (`PATH_NOT_FOUND`, `PROJECT_EXISTS`, `ACCESS_DENIED`, `SCAN_TIMEOUT`, `INVALID_KEY`, `UNREACHABLE`, `RATE_LIMITED`, `TOKEN_LIMIT`, `INTERNAL`)
- WHEN mapping backend codes to frontend codes
- THEN every backend code emitted by the new serializer MUST map deterministically to one frontend code
- AND unknown backend codes MUST collapse safely to `INTERNAL`

#### Scenario: Path-related backend codes map consistently

- GIVEN backend codes `PROJECT_NOT_FOUND` and `FILE_NOT_FOUND`
- WHEN they are received by the frontend
- THEN both MUST map to `PATH_NOT_FOUND`

### Requirement: Logging behavior is preserved

The migration MUST change the IPC error contract without regressing existing logging behavior.

#### Scenario: tracing output remains readable

- GIVEN structured logging that already records backend failures
- WHEN the new error serialization is introduced
- THEN the emitted logs MUST remain human-readable
- AND the migration MUST NOT replace useful log fields with opaque JSON blobs as the only readable representation

### Requirement: Atomic rollout

The contract change MUST ship atomically across backend and frontend.

#### Scenario: Backend serializer and frontend parser land together

- GIVEN the first PR of the migration chain
- WHEN the error contract is changed
- THEN the backend serializer update and the frontend parser/mapping update MUST be merged together
- AND the repo MUST not pass through an intermediate state where backend emits structured payloads but frontend can only consume legacy strings
