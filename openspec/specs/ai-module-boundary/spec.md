# AI Module Boundary Specification

## Purpose

Define the public boundary cleanup required for `engine/src/ai/` during the first hexagonal migration wave.

## Requirements

### Requirement: AI module public surface excludes concrete adapters

The AI module root MUST stop re-exporting concrete adapter implementation details.

#### Scenario: mod.rs exposes only stable public AI contracts

- GIVEN `engine/src/ai/mod.rs`
- WHEN reviewing its public exports after wave 1
- THEN the module MUST expose only stable public AI contracts and utilities needed by external consumers
- AND it MUST NOT re-export concrete adapter types such as `AnthropicProvider`, `ResolvedProvider`, or `ProviderFactory`

#### Scenario: Resolver trait remains available if required by public AIService signatures

- GIVEN the public shape of `AIService`
- WHEN the service requires a resolver trait in its public generic or constructor surface
- THEN the corresponding resolver trait MAY remain public
- BUT concrete resolver implementations MUST stay internal to the AI module boundary

### Requirement: AIService remains the main consumption surface

The rest of the application MUST continue consuming AI behavior through `AIService` rather than concrete provider adapters.

#### Scenario: Tauri layer consumes AIService only

- GIVEN the Tauri backend after wave 1
- WHEN reviewing AI-related command wiring
- THEN commands and app state MUST consume `AIService`
- AND they MUST NOT depend directly on `AnthropicProvider`, `ResolvedProvider`, or `ProviderFactory`

### Requirement: No functional regression in AI behavior

The public boundary cleanup MUST not change AI behavior.

#### Scenario: Existing AI tests continue to pass

- GIVEN the engine AI test suite
- WHEN the boundary cleanup is complete
- THEN existing AI service and provider tests MUST still pass
- AND the cleanup MUST be considered structural only, not behavioral
