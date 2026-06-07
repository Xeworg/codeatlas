# Architecture Review — AI Hexagonal Refactor

**Scope:** Diff in `engine/src/ai/*` and `src-tauri/src/commands.rs`  
**Focus:** Hexagonal alignment, dependency direction, infrastructure leaks.

---

## Correct (what improved)

1. **Presentation → Application service boundary created**  
   `src-tauri/src/commands.rs` no longer directly instantiates `AnthropicProvider`. It now calls `AIService::default()` (lines 673, 755). This is a genuine seam: the presentation layer depends on an application-layer orchestrator instead of an infrastructure adapter.

2. **Port/Adapter pattern introduced**  
   `engine/src/ai/factory.rs` defines `AIProviderResolver` (port) and `ProviderFactory` (adapter). `engine/src/ai/service.rs` parameterizes `AIService<R>` over the resolver, enabling test doubles (verified by `TestResolver` in tests). This matches hexagonal intent.

3. **Error-mapping logic extracted and made pure**  
   `engine/src/ai/anthropic.rs` now has a standalone `map_error_response(status, body)` function. The old inline match-with-body-text logic was harder to unit-test; the extracted version is tested directly in four test cases.

4. **Provider construction decoupled from raw strings**  
   `AnthropicProvider::from_config` and `with_endpoint` centralize endpoint/model defaults. Previously the endpoint URL was a hardcoded literal inside `new()`.

---

## Blocker

None. The diff introduces no critical regressions and does not worsen existing leaks.

---

## Notes & Suggested Fixes

### Severity: MEDIUM — `AIService` instantiated inside command handlers

- **Location:** `src-tauri/src/commands.rs:673` and `:755`  
  `let ai_service = AIService::default();`

- **Problem:** On every `explain_node` and `chat` invocation, the presentation layer constructs the application service (and therefore the concrete `ProviderFactory`). In hexagonal architecture, the application service should be injected at the composition root (`main.rs` / Tauri setup) and held in `AppState`. The command handler should only reach into state.

- **Suggested fix:**
  1. Add an `ai_service` field to `AppState`:
     ```rust
     pub ai_service: Mutex<AIService>,
     ```
  2. Initialize it once in the Tauri `setup` hook.
  3. Change command handlers to:
     ```rust
     let ai_service = state.ai_service.lock().map_err(|e| e.to_string())?;
     ai_service.explain_node(&cfg, ...).await
     ```
     This removes the last on-the-spot infrastructure decision from the presentation layer.

---

### Severity: LOW-MEDIUM — `ResolvedProvider` enum lives in the application layer

- **Location:** `engine/src/ai/resolved.rs`

- **Problem:** The enum lists concrete variants (`Anthropic(AnthropicProvider)`). In strict hexagonal design, the application layer should not know which infrastructure adapters exist. Adding a new provider (e.g., OpenAI) requires editing application-layer code.

- **Context:** This is a pragmatic compromise forced by the associated type `type Provider` on `AIProviderResolver`. It preserves static dispatch.

- **Suggested fix (optional, for stricter hexagonal):**
  - Replace the associated type with a trait object return:
    ```rust
    fn resolve(&self, config: &AIConfig) -> Result<Box<dyn AIProvider>>;
    ```
  - Delete `ResolvedProvider`. Each adapter lives only in its own infrastructure module.
  - If runtime allocation is undesirable, document the compromise explicitly in `resolved.rs`.

---

### Severity: LOW — `ProviderFactory` co-located with the port

- **Location:** `engine/src/ai/factory.rs`

- **Problem:** The file contains both the port (`AIProviderResolver`) and the concrete adapter (`ProviderFactory`). Over time this blurs the boundary between application and infrastructure.

- **Suggested fix:** Move `ProviderFactory` to an `infrastructure` subdirectory (e.g., `engine/src/ai/infrastructure/factory.rs`) while keeping the trait in `engine/src/ai/factory.rs` or renaming it to `resolver.rs`.

---

### Severity: LOW — `ContextBuilder` still used directly in presentation layer

- **Location:** `src-tauri/src/commands.rs` imports `ContextBuilder` from `engine::ai`.

- **Problem:** `ContextBuilder` is arguably an application-layer concern (assembling prompt context). The presentation layer should ideally call `AIService` and let the service invoke `ContextBuilder` internally. This is **pre-existing**; the diff does not make it worse, but it is a leak of application logic into presentation.

- **Suggested fix (future refactor):** Move `ContextBuilder::build_node_context` and `build_chat_context` calls inside `AIService::explain_node` / `AIService::chat`. Pass the raw data (`&graph`, `file_content`, etc.) to the service and let it build context internally.

---

### Severity: INFO — Model default naming inconsistency

- **Location:** `engine/src/ai/anthropic.rs:29` (inside `with_endpoint`)

  ```rust
  model: model.unwrap_or("minimax").to_string(),
  ```

- **Problem:** An `AnthropicProvider` defaults to the model name `"minimax"`. This is a leftover name from a previous provider abstraction and is confusing. Not an architectural leak, but a code-quality issue.

---

## Summary

| Criterion                              | Verdict                                                                                                    |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Moves toward hexagonal design          | **Yes** — application service + resolver port introduced                                                   |
| Dependency direction improved          | **Yes** — presentation no longer imports `AnthropicProvider`                                               |
| Infrastructure leaks into presentation | **Partially fixed** — `AIService` is used, but still constructed on the spot inside commands               |
| Infrastructure leaks into application  | **Minor** — `ResolvedProvider` enum knows concrete adapters; `ProviderFactory` shares a file with the port |
| Testability                            | **Improved** — `TestResolver` + `TestProvider` prove the seam works                                        |

**Bottom line:** The diff is a solid, bounded step toward hexagonal architecture. The most impactful next fix is injecting `AIService` via `AppState` so command handlers stop constructing it.
