# Spec Deltas: pre-wave-2-foundation — frontend-service-layer

> Delta spec for `openspec/specs/frontend-service-layer/spec.md`.
> El refactor pre-wave-2 demuestra que la capa `src/services/*.ts` son 1:1 passthroughs sin lógica cross-cutting. La decisión arquitectónica es **borrar** la capa, no engordarla. Hooks y stores importan directo de `src/lib/tauri-api.ts`.

## REMOVED Requirements

### Requirement: Frontend domain services wrap the Tauri bridge

(Reason: los 5 archivos en `src/services/{ai,project,graph,snapshot,analysis}Service.ts` son re-exports 1:1 de `src/lib/tauri-api.ts`. No agregan lógica cross-cutting, no normalizan errores (esa responsabilidad vive en `tauri-api.ts:58-126`), ni desacoplan la presentación del bridge. Mantenerlos añade una capa a navegar y un segundo lugar a actualizar cuando `tauri-api.ts` cambia.)
(Migration: los hooks (`useAI`, `useAIConfig`, `useGraph`, `useNodeDetails`, `useNodeOutline`, `useProject`, `useArchitecture`, `useExport`) y el store (`useSnapshotStore.ts`) importan directo de `@/lib/tauri-api`. El test `src/services/__tests__/services-boundary.test.ts` se renombra a `src/lib/__tests__/tauri-api-bridge.test.ts` y se reduce a un test del parser de errores + un smoke test de la superficie de `tauri-api` + un test de "no module imports from a deleted `src/services/*` path". Ver el requirement añadido "services-boundary test becomes tauri-api bridge test" abajo.)

#### Scenario: developer looks for a service facade

- **WHEN** el developer busca una fachada adaptadora para una operación de IA
- **THEN** importa directo desde `src/lib/tauri-api.ts` (ej. `import { explainNode } from '@/lib/tauri-api'`)
- **AND** no existe `src/services/aiService.ts` envolviendo la llamada

#### Scenario: no leftover import from src/services

- **WHEN** se ejecuta `rg "from ['\"]@/services" src/hooks/ src/stores/ src/components/`
- **THEN** retorna 0 hits
- **AND** el alias `@/services` (si existe en `tsconfig.json`) no se usa en código de runtime

## ADDED Requirements

### Requirement: hooks consume tauri-api directly

All `src/hooks/*.ts` and `src/stores/useSnapshotStore.ts` MUST import Tauri commands directly from `src/lib/tauri-api.ts`. The intermediate `src/services/*` layer is gone. The error-normalization helper (`toApiError`, `getErrorMessage`) still lives in `tauri-api.ts` and is the single point of structured-error parsing — hooks do not re-implement it.

#### Scenario: hook import points to tauri-api

- **WHEN** `useAI`, `useAIConfig`, `useGraph`, `useNodeDetails`, `useNodeOutline`, `useProject`, `useArchitecture`, `useExport` are read
- **THEN** every `import` for a backend operation resolves to `@/lib/tauri-api` (or relative equivalent `../lib/tauri-api`)
- **AND** `rg "from ['\"]@/services" src/hooks/ src/stores/` returns 0 hits

#### Scenario: stores import tauri-api directly

- **WHEN** `src/stores/useSnapshotStore.ts` is read
- **THEN** any snapshot backend call is `import { listSnapshots, createSnapshot, getSnapshot } from '@/lib/tauri-api'`
- **AND** no snapshot import path resolves to a deleted `src/services/snapshotService.ts`

#### Scenario: components still consume hooks, not tauri-api

- **WHEN** any file under `src/components/` is read
- **THEN** it does NOT import `tauri-api.ts` directly — it consumes hooks or stores, preserving the boundary that pre-wave-2 already established for components

### Requirement: services-boundary test becomes tauri-api bridge test

The file `src/services/__tests__/services-boundary.test.ts` is renamed to `src/lib/__tests__/tauri-api-bridge.test.ts` and reduced to:

- 1 test that the parser handles structured `IpcErrorPayload` correctly
- 1 smoke test that the renamed API surface still works (representative commands, e.g. `explainNode`, `getGraph`, `listSnapshots`)
- 1 test that no other module imports from a deleted `src/services/*` path (acts as a static guard analogous to the backend `check:arch`)

#### Scenario: services-boundary test file is gone

- **WHEN** `find src/services -name "*.test.ts"` is run
- **THEN** 0 files are returned

#### Scenario: tauri-api-bridge test covers the parser

- **WHEN** the new test file is read
- **THEN** it contains at least one test for the structured `IpcErrorPayload` path
- **AND** at least one test for the legacy fallback (now unreachable for normal flow, defensively kept during the rollout window)

#### Scenario: tauri-api-bridge test guards against service-layer resurrection

- **WHEN** the static-guard test inside the file is read
- **THEN** it asserts that no `src/**/*.ts` file outside `src/lib/tauri-api.ts` itself imports from a `src/services/*` path
- **AND** reintroducing `src/services/aiService.ts` and importing it from a hook makes the test fail
