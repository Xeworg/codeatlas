# Wave 2 hexagonal completion — design

## Resumen

Spanish-language design.md (872 líneas, 8 ADs, 8-PR chain) para el change `wave-2-hexagonal-completion`. Resuelve 8 decisiones de arquitectura left open por las 5 specs. Input directo de `sdd-tasks`.

## Las 8 decisiones de arquitectura (AD-001 a AD-008)

1. **AD-001 — Renombre `AnalysisRepository` → `AnalysisDataSource`**: el nuevo port (hexagonal-ports spec) absorbe el rol del viejo. Un solo trait = un solo path. Resuelve issue #5 del spec phase.

2. **AD-002 — Estructura de archivos para los 5 nuevos ports**: `engine/src/ports/hexagonal.rs` (nuevo módulo) contiene Clock, IdGenerator, Stopwatch, FileSourceReader. `AnalysisDataSource` se queda en `ports.rs` (renombre, no nuevo dominio). 4 ports cross-cutting en submódulo separado.

3. **AD-003 — Patrón de inyección dual**: `Arc<dyn ...>` en AppState (4 nuevos campos) + genéricos en services (`ScanService<S, A, C, I, W, F>`). Mocks `pub(crate)`. Precedente: pre-wave-2 PR-B.

4. **AD-004 — CQRS split con 2 adapters separados**: `QueryRepositoryAdapter` y `CommandRepositoryAdapter` (cada uno wrappea `Arc<DbPool>` clonado). Enforce compilación-time de read/write separation. 4 wave-1 traits persisten como fachadas delgadas sobre CQRS.

5. **AD-005 — 5 domain types nuevos en `engine/src/models/workspace.rs`**: `WorkspaceMeta`, `SnapshotMeta`, `CommentMeta`, `HealthRecord`, `C4View`. Más movimiento de `ExecutiveSummary`/`SnapshotDiff`/`C4View` desde `db::queries` a `models::workspace`. 9 firmas tuple-typed → 5 domain types.

6. **AD-006 — C3a refactor con 2 métodos de `AIService`**: `prepare_explain_context` y `prepare_chat_context` retornan `ExplainContext`/`ChatContext` DTOs. `ContextBuilder` se mueve a `pub(crate)`. `commands.rs::explain_node`/`chat` se reducen de 80 a ~30 líneas cada uno.

7. **AD-007 — Arch-guard patterns concretos (strict purity)**: 8 nuevos regex patterns en `scripts/ci/check-architecture.mjs` (ENGINE_STD_FS_COMMANDS, ENGINE_ANALYSIS_DIRECT, ENGINE_CHRONO_NOW_SERVICES, ENGINE_UUID_NEW_V4_SERVICES, ENGINE_INSTANT_NOW_SERVICES, ENGINE_DBQUERY_LEAK, ENGINE_STD_FS_SERVICES, FRONTEND_TOUSERMESSAGE_TAURI). 4 nuevos self-test fixtures. Allowlist explícita en script (composition root en `lib.rs` no se flaggea).

8. **AD-008 — Plan de implementación por PR**: 8 PRs feature-branch-chain (C1–C7 con C3 split en C3a+C3b). C1 (250-350L, 🟢), C2 (450-600L, 🟡), C3a (400-550L, 🔴 size:exception), C3b (250-400L, 🟡), C4 (800-1000L, 🟡 size:exception), C5 (500-700L, 🟡 size:exception), C6 (600-800L, 🟢), C7 (1000-2000L, 🟡 size:exception, sin línea dura). Total 4250-6750 líneas, dentro del budget 800 con flex autorizado por usuario (observación #673).

## Files tocados por el design

- `openspec/changes/wave-2-hexagonal-completion/design.md` (new, 872 líneas)

NO se crearon code-stubs — el design es puramente arquitectura, los type signatures concretos (AD-005, AD-006) van inline en el markdown.

## 8-PR → AD mapping

- **C1** implementa: AD-001 (renombre AnalysisDataSource), partial AD-002 (pub(crate) de 5 traits en ports.rs), command-bridge (3 dead commands).
- **C2** implementa: AD-002 (4 ports en hexagonal.rs), AD-003 (Arc<dyn> en AppState, genéricos en services).
- **C3a** implementa: AD-006 (extract use cases), partial AD-001 (AnalysisDataSource adapter).
- **C3b** implementa: error-contract deltas (T4 string literals, F2 errors.ts).
- **C4** implementa: AD-005 (9 firmas tuple→domain).
- **C5** implementa: AD-003 (FileSourceReader en GraphService), análisis service-level ports.
- **C6** implementa: AD-007 (8 arch-guard patterns), docs, coverage gate.
- **C7** implementa: AD-004 (CQRS split con 2 adapters).

## Open questions para el usuario (mínimas)

- AD-001 (renombre) — cambio irreversible en C1, validar nombre.
- AD-004 (CQRS split) — 2 adapters separados, 80 líneas extra vs combined. Validar.
- AD-005 (9 domain types en C4) — el PR más grande. Validar size:exception.
- AD-008 (C3 split) — 8 PRs vs 7 originales. Más overhead pero menos riesgo per-PR.

Los 7 issues del spec phase (issue #1-#7) están RESUELTOS o DIFERIDOS — no bloquean `sdd-tasks`.