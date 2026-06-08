# Review: AI Hexagonal Migration Slice

## Scope

Factory/service/resolution flow, commands wiring, and Anthropic compatibility behavior.

## Build / Test Evidence

- `engine` AI tests: **23 passed, 0 failed** (all `ai::anthropic`, `ai::factory`, `ai::service`, `ai::context`)
- `src-tauri` tests: **31 passed, 0 failed**
- `cargo check -p engine` and `cargo check -p codeatlas` both **clean** (no warnings, no errors)

---

## Correct

- **Factory resolution is correct.** `ProviderFactory` resolves `"anthropic"` and `"custom"` to `ResolvedProvider::Anthropic(AnthropicProvider::from_config(...))`. The `keeps_custom_as_compatibility_alias_during_migration` test explicitly locks this behavior.
  - File: `engine/src/ai/factory.rs:33`
- **Commands are properly wired to `AIService`.** `src-tauri/src/commands.rs` replaces direct `AnthropicProvider::new(&cfg.api_key, Some(&cfg.model))` with `AIService::default().explain_node(&cfg, ...)` and `AIService::default().chat(&cfg, ...)`. No remaining references to `AnthropicProvider` or `AIProvider` in `src-tauri/src`.
  - File: `src-tauri/src/commands.rs:673`, `src-tauri/src/commands.rs:755`
- **Custom endpoints now actually work.** `AnthropicProvider::from_config` passes `config.endpoint` through `with_endpoint`, which previously was ignored. The `provider_creation_with_custom_endpoint` test confirms this.
  - File: `engine/src/ai/anthropic.rs:37-44`, `engine/src/ai/anthropic.rs:163-171`
- **Error mapping is functionally preserved.** `map_error_response` centralizes the status→`AppError` logic and is tested for 401, 429, 400-with-token, and 500. The old match logic in `request()` is replaced by a cleaner, equivalent extraction.
  - File: `engine/src/ai/anthropic.rs:7-18`, `engine/src/ai/anthropic.rs:72-76`
- **ResolvedProvider enum dispatch is clean.** `ResolvedProvider` forwards `explain_node` and `chat` to the inner `AnthropicProvider`. Since `AnthropicProvider` is `Send + Sync` (only `String` fields), `ResolvedProvider` auto-satisfies the `AIProvider: Send + Sync` bound.
  - File: `engine/src/ai/resolved.rs:13-34`
- **AIService generic design is sound.** `AIService<R>` only exposes `explain_node`/`chat` when `R: AIProviderResolver`. The default `AIService<ProviderFactory>` is correct, and the test suite uses a `TestResolver` to verify the seam.
  - File: `engine/src/ai/service.rs:28-48`

---

## Fixed

- **None required.** The diff is correct and tests pass.

---

## Blocker

- **None.** No critical issues or regressions.

---

## Note

1. **Missing test coverage for 403.** `map_error_response` handles `401 | 403` as `InvalidApiKey`, but only 401 is tested. 403 should be added to `error_mapping_invalid_api_key` or as a separate test.
   - File: `engine/src/ai/anthropic.rs:177-180`
   - Severity: Low
   - Suggested fix: Add `assert!(matches!(map_error_response(403, "forbidden"), AppError::InvalidApiKey));` to the test.

2. **Missing test coverage for 400 without "token".** `map_error_response` falls through to `AIUnavailable` when body lacks `"token"`, but this path is not explicitly tested.
   - File: `engine/src/ai/anthropic.rs:7-18`
   - Severity: Low
   - Suggested fix: Add a test `error_mapping_400_without_token` asserting `AIUnavailable`.

3. **Empty endpoint for `"custom"` provider now causes empty-URL requests.** The old code ignored `config.endpoint` entirely (hardcoded Anthropic URL). The new code uses `config.endpoint`. The frontend (`ApiKeySetup.tsx`) does not validate that the endpoint is non-empty when `provider === 'custom'`, so `endpoint: Some("")` could be sent and would be used as the request URL.
   - File: `engine/src/ai/anthropic.rs:37-44`, `src/components/onboarding/ApiKeySetup.tsx:53-56`
   - Severity: Low (behavior change, not regression; previously a silent bug where endpoint was ignored)
   - Suggested fix: Either add `!endpoint.is_empty()` guard in `with_endpoint` (`endpoint.filter(|e| !e.is_empty())`) or add frontend validation for non-empty endpoint when custom is selected.

4. **Default model name is `"minimax"` for an Anthropic provider.** This is pre-existing, not introduced by the diff, but worth noting as a mismatch.
   - File: `engine/src/ai/anthropic.rs:40`
   - Severity: Cosmetic
   - Suggested fix: Change to `"claude-sonnet-4-20250514"` or another Anthropic default.

5. **`.pi/settings.json` and `.pi/agents/sdd-apply.md` changes are out of scope.** These are harness model overrides (e.g., `reviewer` switched to `kimi-k2.6`, `sdd-apply` to `MiniMax-M2.7`). They do not affect the correctness of the AI hexagonal migration slice.
