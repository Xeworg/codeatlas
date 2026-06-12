# Wave 2 C3a — tasks

> **Posición en la cadena**: 3° de 8 PRs (Wave 2). C1 ✅ (`be33134`, PR #9), C2 ✅ (`fe26f70`, PR #10).
> **Base branch**: `main` @ `fe26f70` (post-C2).
> **Branch propuesto**: `feat/wave-2-c3a-ai-context-prep`.
> **PR title propuesto**: `[Wave 2 C3a] AI context preparation + AnalysisDataSource.list_files_for_project + chat double-push fix`.

## Resumen

| Campo | Valor |
|-------|-------|
| Total tasks | 7 (6 implementación + 1 verify) |
| Líneas estimadas | 340-470 (netas, contando tests y borrados de `commands.rs`) |
| Risk | 🔴 HIGH (4 factores acumulativos — ver §Riesgos) |
| `size:exception` | **REQUERIDO** (autorizado por preflight #673) |
| Chain strategy | feature-branch-chain |
| Decisión previa a `sdd-apply` | **No** (user pre-autorizó flex en preflight #673) |

## Review Workload Forecast

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

| Campo | Valor |
|-------|-------|
| Líneas estimadas (netas) | 340-470 |
| 400L budget risk | High (excede 400) |
| Chained PRs recommended | Yes (C3a es 3° de 8 PRs encadenados; cadena es mandatoria) |
| Suggested split | single-PR C3a con `size:exception` (no se fragmenta limpiamente) |
| Delivery strategy | `exception-ok` (preflight #673 autorizó flex) |
| Chain strategy | feature-branch-chain (C3a branch de `main`; C3b branch del PR-C3a) |

### Suggested Work Units (commits)

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| WU-1 | `feat(ai): add ExplainContext DTO + AIService::prepare_explain_context` (C3a.1) | PR-C3a | incluye ~10L spec delta de `ProviderFactory` en `hexagonal-ports/spec.md` (Q3=A) |
| WU-2 | `feat(ai): add ChatContext DTO + AIService::prepare_chat_context` (C3a.2) | PR-C3a | simétrico a WU-1 |
| WU-3 | `refactor(commands): reduce explain_node to shim delegating to prepare_explain_context` (C3a.3) | PR-C3a | test-adaquation; ≤30L |
| WU-4 | `fix(commands): reduce chat to shim and fix double-push bug` (C3a.4) | PR-C3a | `fix:` por el bug fix; regression test obligatorio |
| WU-5 | `refactor(ai): demote ContextBuilder to pub(crate)` (C3a.5) | PR-C3a | AI1; cambio de visibilidad |
| WU-6 | `feat(analysis): add AnalysisDataSource::list_files_for_project` (C3a.6) | PR-C3a | partial AD-001; ortogonal a WU-1..5 |
| WU-7 | `docs(spec): document ProviderFactory as public engine API` (Q3 spec delta) | PR-C3a | ~10L en `hexagonal-ports/spec.md` |

7 commits esperados, todos dentro del budget `size:exception` (~340-470L).

## Per-task breakdown

### C3a.1 — `AIService::prepare_explain_context` + DTO `ExplainContext`

- **AD reference**: AD-006
- **Files touched**:
  - `engine/src/ai/service.rs` (nuevo método + DTO + tests en mod tests)
  - `engine/src/ai/mod.rs` (export `ExplainContext`)
  - `openspec/specs/hexagonal-ports/spec.md` (Q3 spec delta: ~10L documentando `ProviderFactory` como public API de `engine::ai` — sub-deliverable de C3a.1)
- **Test strategy**: RED-first
  - RED: `prepare_explain_context` retorna `ExplainContext` con campos correctos (node, neighbors, snapshot)
  - RED: `ExplainContext` serializa a camelCase JSON (test de serde)
  - RED: usa `MockClock` y `MockIdGen` (puertos C2) para IDs/timestamps deterministas
- **Dependencies**: puertos C2 ya en main (`engine/src/ports/hexagonal.rs`)
- **Done criteria**:
  - [x] `prepare_explain_context` firma acordada (sync/async, returns `Result` o `ExplainContext`)
  - [x] DTO `ExplainContext` con `#[serde(rename_all = "camelCase")]`
  - [x] 3+ unit tests pasando
  - [x] `cargo test --tests` green
  - [x] Q3 spec delta: ~10L en `hexagonal-ports/spec.md` documenta `ProviderFactory` como public API

### C3a.2 — `AIService::prepare_chat_context` + DTO `ChatContext`

- **AD reference**: AD-006
- **Files touched**:
  - `engine/src/ai/service.rs` (nuevo método + DTO + tests)
  - `engine/src/ai/mod.rs` (export `ChatContext`)
- **Test strategy**: RED-first
  - 3+ unit tests con `MockClock`/`MockIdGen` para determinismo
  - Test: el user message se empuja **una sola vez** (foundation del fix C3a.4)
- **Dependencies**: puertos C2
- **Done criteria**:
  - [x] `prepare_chat_context` firma acordada
  - [x] DTO `ChatContext` con `#[serde(rename_all = "camelCase")]`
  - [x] 3+ unit tests
  - [x] `cargo test --tests` green

### C3a.3 — `commands.rs::explain_node` shim refactor

- **AD reference**: AD-006
- **Files touched**: `src-tauri/src/commands.rs:344-400` (refactor solamente)
- **Test strategy**: test-adaquation (tests integration existentes deben seguir pasando)
- **Dependencies**: C3a.1
- **Done criteria**:
  - [x] `explain_node` ≤ 30 líneas
  - [x] Delega a `AIService::prepare_explain_context`
  - [x] IPC contract preservado (mismo return type en la frontera)
  - [x] `cargo test` green; `src-tauri/src/commands.rs` compila

### C3a.4 — `commands.rs::chat` shim refactor + double-push fix

- **AD reference**: AD-006 + bug fix
- **Files touched**:
  - `src-tauri/src/commands.rs:401-490` (refactor + fix)
  - `engine/src/ai/service.rs` (eliminar push redundante en `chat_with_context` si lo hubiera)
- **Test strategy**: RED-first para el fix + test-adaquation para el shim
  - **Regression test obligatorio**: `full_history` contiene **exactamente 1** user message tras invocar `chat`
  - Test: `CapturingProvider` (precedente: `engine/src/ai/service.rs:493`) registra la lista exacta de `ChatMessage` que recibe el provider
- **Dependencies**: C3a.2
- **Done criteria**:
  - [x] `chat` ≤ 30 líneas
  - [x] Delega a `AIService::prepare_chat_context`
  - [x] **Double-push bug FIXED**: user message aparece exactamente 1 vez
  - [x] Regression test presente y passing
  - [x] IPC contract preservado

### C3a.5 — `ContextBuilder` → `pub(crate)`

- **AD reference**: AI1
- **Files touched**:
  - `engine/src/ai/mod.rs:12` (quitar `pub use context::ContextBuilder;`)
  - `engine/src/ai/context.rs` (cambiar visibilidad en sitio si es necesario; `ContextBuilder` debe quedar `pub(crate)` o `pub(super)`)
- **Test strategy**: compile-time check + test-adaquation
  - Pre-check: `rg "ContextBuilder" /home/xeworg/Proyectos/codeatlas/src-tauri/` debe retornar 0 hits (verificado en explore #690)
  - `cargo build` en ambos crates
- **Dependencies**: ninguna (ortogonal a C3a.1-C3a.4)
- **Done criteria**:
  - [x] `ContextBuilder` es `pub(crate)` o `pub(super)`
  - [x] Todos los callers internos al crate `engine` siguen compilando
  - [x] 0 referencias externas en `src-tauri/src/`
  - [x] `cargo build` green en `engine/` y `src-tauri/`
  - [x] `rg "pub use.*ContextBuilder" engine/src` retorna 0 hits

### C3a.6 — `AnalysisDataSource::list_files_for_project`

- **AD reference**: partial AD-001 (Q1=A)
- **Files touched**:
  - `engine/src/ports.rs:603` (nuevo método en trait `AnalysisDataSource`)
  - `engine/src/ports.rs:662-680` (impl en `AnalysisDataSourceAdapter`)
  - `engine/src/ports.rs:901-910` (impl en `Arc<dyn AnalysisDataSource>`)
  - `engine/src/services/analysis_service.rs:195, 233, 260` (refactor: usar `list_files_for_project` en lugar de `analysis_repo.pool()` para al menos 1 call site de `get_architecture_detection`)
- **Test strategy**: RED-first
  - Test 1: `list_files_for_project(known_id)` retorna `Vec<FileMeta>` esperado
  - Test 2: `list_files_for_project(unknown_id)` retorna `Vec` vacío
  - Test 3: integration test con sqlite-backed mock
- **Dependencies**: ninguna (ortogonal; puede hacerse en paralelo con C3a.1-C3a.5)
- **Done criteria**:
  - [x] `list_files_for_project(project_id: &str) -> Vec<FileMeta>` en el trait
  - [x] SQLite adapter impl (en `AnalysisDataSourceAdapter` y en `Arc<dyn>`)
  - [x] 2+ unit tests + 1 integration test
  - [x] `cargo test --tests` green
  - [x] `rg "analysis_repo\.pool" engine/src/services/analysis_service.rs` baja de 3 hits a 1-2

### C3a.7 — Verificación + PR

- **Tipo**: meta (no es un commit de código)
- **Test strategy**: standard verification suite
  - `cd engine && cargo test --tests` — green (258+ tests post-C2 + nuevos de C3a)
  - `cd src-tauri && cargo build` — green
  - `cd engine && cargo clippy -- -D warnings` — sin nuevos warnings
  - `cd engine && cargo fmt --check` — green
  - `npm run check:arch` — sin violaciones (dual-language arch guard)
  - `npm run lint && npm run typecheck` — green
- **Dependencies**: C3a.1 - C3a.6
- **Done criteria**:
  - [x] Las 6 impl tasks + WU-7 docs commit pushed a `feat/wave-2-c3a-ai-context-prep`
  - [x] Dual review pasada (Judge A + Judge B) — uso de `judgment-day`
  - [x] Round 1 review fixes aplicados (esperar ~2-5 fixes por precedent C2)
  - [x] PR #11 abierto contra `main` con title `[Wave 2 C3a] AI context preparation + AnalysisDataSource.list_files_for_project + chat double-push fix`
  - [x] User (sole reviewer) merged

## Dependencies graph

```text
                    ┌─ C3a.1 (prepare_explain_context) ──> C3a.3 (explain_node shim)
C2 ports (DONE) ───┤
                    └─ C3a.2 (prepare_chat_context) ─────> C3a.4 (chat shim + bugfix)

C3a.5 (ContextBuilder pub(crate))  ── independiente (orthogonal)
C3a.6 (list_files_for_project)     ── independiente (orthogonal)

C3a.7 (verify + PR)  ── depende de C3a.1..C3a.6
```

Notas:
- C3a.1 y C3a.2 son independientes entre sí (ambos dependen solo de C2).
- C3a.3 depende de C3a.1; C3a.4 depende de C3a.2.
- C3a.5 y C3a.6 son ortogonales a los demás y pueden hacerse en paralelo.
- C3a.7 es el último paso; depende de todos los anteriores.

## TDD cycle mapping

| Task | Cycle | Notas |
|------|-------|-------|
| C3a.1 | RED-first | Nuevo método + nuevo DTO; 3+ tests unitarios con `MockClock`/`MockIdGen` |
| C3a.2 | RED-first | Nuevo método + nuevo DTO; 3+ tests; test del single-push sienta base para C3a.4 |
| C3a.3 | test-adaquation | Refactor de signature existente; tests integration existentes cubren comportamiento |
| C3a.4 | RED-first (fix) + test-adaquation (shim) | **Regression test obligatorio del double-push**; refactor del shim por su parte es test-adaquation |
| C3a.5 | test-adaquation | Cambio de visibilidad; `cargo build` + grep son la verificación |
| C3a.6 | RED-first | Nuevo método en trait + impl; 2+ unit + 1 integration test |
| C3a.7 | meta (verify) | Standard verification suite + dual review |

## Verification commands

```bash
# Engine tests + builds
cd engine && cargo test --tests          # 258+ tests pasan + nuevos
cd engine && cargo build
cd engine && cargo clippy -- -D warnings  # sin nuevos warnings
cd engine && cargo fmt --check

# src-tauri build
cd src-tauri && cargo build

# Arch guard + frontend
npm run check:arch                         # dual-language, sin violaciones
npm run lint && npm run typecheck

# Greps MUST-return-zero
rg "pub use.*ContextBuilder" engine/src                                # 0 hits (C3a.5)
rg "std::fs::read_to_string" src-tauri/src/commands.rs                  # 0 hits en explain_node/chat
rg "full_history\.push" src-tauri/src/commands.rs                       # 0 hits en chat (C3a.4)
rg "full_history\.push" engine/src/ai/service.rs                        # 0 hits en chat_with_context (C3a.4)
rg "analysis_repo\.pool" engine/src/services/analysis_service.rs       # baja de 3 a 1-2 (C3a.6)
```

## Out of scope (deferred)

Q4=A: todo lo siguiente queda **fuera** de C3a y se difiere a C3b+, cleanup PR, o wave 3:

1. Cierre de los 3 string literals en `src-tauri/src/commands.rs:271, 278, 329` → **C3b (T4)**.
2. Extracción de `toUserMessage`/`getErrorMessage` a `src/lib/errors.ts` → **C3b (F2)**.
3. 9 domain types en `WorkspaceRepository` (D1/D2) → **C4**.
4. Real `AnalysisDataSource` para todo `analysis/*` (P1 completo) → **C5**.
5. `FileSourceReader` port (S1), `FileWalker`/`ParserRegistry` ports (S2), `AnalysisService` unit tests profundos (S5) → **C5**.
6. CQRS split de `ProjectRepository` → **C7**.
7. Strict arch-guard patterns (8 regex nuevos) → **C6**.
8. `pub(crate)` migration de los 6 port traits → **wave 3** (ADR-009, observación #681). C3a.5 es la **única** excepción permitida.
9. Refactor de `detect_architecture(project_id, &DbPool)` → `detect_architecture(files: &[FileMeta])` (Q1=B) → **diferido**.
10. `scan_started_at` end-to-end → **diferido** (cleanup o wave 3+).
11. `MockIdGen` nil-on-first-call semantics, `make_scan_service` consolidation, naming consistency `IdGenerator`/`RandomIdGen` → **diferido** (cleanup).

## File map (expected)

| Task | Archivos creados | Archivos modificados | Líneas netas est. |
|------|------------------|----------------------|-------------------|
| C3a.1 | (ninguno) | `engine/src/ai/service.rs`, `engine/src/ai/mod.rs`, `openspec/specs/hexagonal-ports/spec.md` | +60-90 (incluye ~10L spec delta) |
| C3a.2 | (ninguno) | `engine/src/ai/service.rs`, `engine/src/ai/mod.rs` | +60-90 |
| C3a.3 | (ninguno) | `src-tauri/src/commands.rs` (refactor) | -50 / +30 (neto −20) |
| C3a.4 | (ninguno) | `src-tauri/src/commands.rs`, `engine/src/ai/service.rs` (eliminar push redundante) | -60 / +40 + regression test (neto −20) |
| C3a.5 | (ninguno) | `engine/src/ai/mod.rs:12`, `engine/src/ai/context.rs` | ±0-5 |
| C3a.6 | (ninguno) | `engine/src/ports.rs`, `engine/src/services/analysis_service.rs` | +50-70 |
| C3a.7 | (ninguno) | (no code; PR template + verification) | (meta) |
| **Total** | (ninguno) | — | **340-470 netas** |

## Riesgos y mitigaciones (resumen)

C3a es **🔴 HIGH risk** por 4 factores acumulativos (per #693):

1. **Toca superficie IPC pública** — `commands.rs::explain_node` y `chat` son hot paths del frontend. **Mitigación**: tests integration con `MockClock`/`MockIdGen` que ejerciten el camino completo.
2. **Cross-crate surface** — `ExplainContext` y `ChatContext` se vuelven API público de `engine::ai`. **Mitigación**: tests `serde_json::to_string` que asserten camelCase estable.
3. **Refactor + extracción simultáneos** sobre ~135 líneas con un bug pre-existente embebido. **Mitigación**: test con `CapturingProvider` (precedente `engine/src/ai/service.rs:493`).
4. **Bug pre-existente del double-push** — `commands.rs::chat:458` + `AIService::chat_with_context:158` ambos empujan. **Mitigación**: regression test obligatorio; solo `prepare_chat_context` empuja.

## Siguiente paso

Usuario revisa este `tasks.md`. Si aprueba:

1. Delegar a `sdd-apply` con el bloque de tareas C3a.1-C3a.6 + la spec delta WU-7.
2. `sdd-apply` ejecuta los 7 work-unit commits en `feat/wave-2-c3a-ai-context-prep`.
3. `sdd-apply` corre la suite de verificación (C3a.7).
4. Dual review con `judgment-day` (Judge A + Judge B).
5. Round 1 review fixes aplicados.
6. PR #11 abierto contra `main`.
7. User (sole reviewer) mergea.

**Status**: ready for sdd-apply (sujeto a aprobación del usuario).
