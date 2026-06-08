# Frontend Service Layer Specification

## Purpose

Define the frontend service and hook layer that removes direct bridge usage from `App.tsx` and React components during the first hexagonal migration wave.

## Requirements

### Requirement: Frontend domain services wrap the Tauri bridge

The frontend MUST expose domain-oriented service modules that encapsulate bridge calls and error normalization.

#### Scenario: Domain services exist for the main frontend responsibilities

- GIVEN the frontend source tree after wave 1
- WHEN inspecting `src/services/`
- THEN there MUST be service modules covering at least:
  - project/scan flow
  - graph flow
  - workspace-related flow
  - AI flow
- AND those modules MUST own bridge invocation and backend error normalization

#### Scenario: Services return typed errors

- GIVEN a failed bridge call
- WHEN a frontend service handles it
- THEN the service MUST normalize the failure through the structured error contract
- AND the propagated error shape MUST remain compatible with `ApiError`

### Requirement: Hooks own orchestration

The frontend MUST move orchestration out of `App.tsx` and components into hooks.

#### Scenario: Core hooks exist for application orchestration

- GIVEN the frontend hooks layer after wave 1
- WHEN inspecting `src/hooks/`
- THEN there MUST be hook coverage for the main orchestration responsibilities currently spread across `App.tsx` and components
- AND the primary hooks MUST include `useProject` and `useGraph`
- AND workspace/AI responsibilities MUST be exposed through dedicated hooks or clearly isolated service consumers

#### Scenario: Hooks manage loading and error state

- GIVEN an async hook-driven operation
- WHEN consuming it from a component
- THEN the hook MUST expose loading and error state explicitly
- AND the component MUST not need to duplicate orchestration state machines inline

### Requirement: Components stop importing tauri-api directly

React components MUST no longer depend directly on `src/lib/tauri-api.ts`.

#### Scenario: No direct bridge imports remain in components

- GIVEN any file under `src/components/`
- WHEN reviewing its imports
- THEN it MUST NOT import `tauri-api.ts` directly
- AND it MUST consume hooks or domain services instead

#### Scenario: App.tsx stops invoking bridge calls inline

- GIVEN `src/App.tsx` after the migration
- WHEN reviewing the component body
- THEN the component MUST not contain direct bridge calls or multi-step orchestration logic inline
- AND it MUST primarily compose hooks, UI layout, and event wiring

### Requirement: Bridge normalization remains centralized

The frontend MUST keep bridge-level normalization centralized even after services are introduced.

#### Scenario: Structured error parsing is not reimplemented per component

- GIVEN multiple service modules
- WHEN they handle backend failures
- THEN they MUST reuse the shared bridge/error normalization path
- AND components MUST not parse backend error payloads themselves
