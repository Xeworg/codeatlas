# Tasks: pre-wave-2-foundation

> 2 PRs encadenados. PR-A instala cimientos (5 tasks, ~150-200 líneas). PR-B ejecuta la mudanza (14 tasks, ~400-500 líneas).

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines (PR-A) | ~150-200 |
| Estimated changed lines (PR-B) | ~400-500 |
| 400-line budget risk (PR-A) | Low |
| 400-line budget risk (PR-B) | High |
| Chained PRs recommended | Yes (PR-A → PR-B) |
| Suggested split | PR-A (5 tasks) → PR-B (14 tasks) |
| Delivery strategy | auto-chain |
| Chain strategy | feature-branch-chain |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

---

## PR-A — foundations (5 tasks, ~150-200 líneas)

Branch: `feat/pre-wave-2-pr-a-foundations` (existe desde `main@04e4c73`).

### Task A.1 — Tighten port trait visibility to `pub(crate)`

- Footprint: `engine/src/ports.rs:37, 139, 224, 499, 607` (5 traits); `engine/tests/workspace_service_test.rs:20` (escape hatch).
- Action: cambiar 5 traits `pub` → `pub(crate)`; NO tocar adapters (líneas 68, 162, 335, 533, 636). Mover el test de `workspace_service_test.rs:20` (única línea que importa `WorkspaceRepository` directamente) a `mod tests` de `engine/src/services/workspace_service.rs`.
- Test: `rg "use engine::ports::(Scan|Graph|Workspace|Analysis)Repository\b" engine/src-tauri/ src-tauri/` → 0 hits; `cargo check` desde `engine/` y `src-tauri/`.
- Spec: `port traits are crate-internal`.
- Risk: 🟡 Medium (escape hatch del integration test).

### Task A.2 — Create CI architecture guard script

- Footprint: `scripts/ci/check-architecture.mjs` (nuevo, ~80 líneas), `scripts/ci/__fixtures__/{forbidden,clean}.rs` (nuevos), `package.json` (agregar script), `.github/workflows/ci.yml` (agregar step en job `rust-backend`).
- Action: script Node con 6 patterns prohibidos (tabla design D4); exit 1 ante cualquier violación; sub-comando `--self-test` con fixtures. Política ESTRICTA (NO honrar `// arch-allow`).
- Test: `node scripts/ci/check-architecture.mjs --self-test` → exit 0; reintroducir fuga temporal → exit 1; `// arch-allow` arriba de la fuga → exit 1.
- Spec: `CI guard blocks port leakage` (3 scenarios).
- Risk: 🟢 Low.

### Task A.3 — Dedupe error-mapping helpers

- Footprint: `src-tauri/src/commands.rs:475-501` (borrar helpers), `engine/src/services/scan_service.rs:320, 325` (canónica, ya existe), `src-tauri/src/commands/tests/observability_tests.rs:1-83` (mover 7 tests a `engine/src/services/scan_service.rs::tests`).
- Action: borrar `is_root_path_conflict` y `map_save_scan_result_error` de `commands.rs`; mover 7 tests al `mod tests` del service; agregar 1 test estático (`include_str!` + assert) que pruebe que `commands.rs` ya no contiene esos símbolos.
- Test: `cargo test` desde `engine/` y `src-tauri/`; test estático verde.
- Spec: design decisión 6 (error-mapping dedup, opción d).
- Risk: 🟡 Medium (cambio de mensaje frontend: `ProjectNotFound` en vez de string user-facing).

### Task A.4 — Document `Arc<Mutex<...>>` lifetime contract

- Footprint: `src-tauri/src/commands.rs:24-38` (espejar docstring de `engine/src/ports.rs:627-635`).
- Action: agregar docstring a `AppState` explicando: por qué `Arc<Mutex<T>>` (compartir ownership con `AppStatePortAdapter`), contrato de lifetime, requisito `Send + Sync` auto-derivado, referencia cruzada.
- Test: `cargo doc --no-deps --document-private-items` sin warnings. Sin test funcional.
- Spec: design decisión 5.
- Risk: 🟢 Low.

### Task A.5 — PR-A final verification

- Action: `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test` (engine/ y src-tauri/); `npm run lint` + `npm run typecheck` + `npm run test` + `npm run check:arch -- --self-test`; `git diff --stat main..HEAD` ≤ 200 líneas; `rg "fn (is_root_path_conflict|map_save_scan_result_error)" src-tauri/src/commands.rs` → 0 hits.
- Acceptance: todos los gates verdes; diff ≤ 200; listo para abrir PR.
- Spec: criteria PR-A (proposal:80-87).
- Risk: 🟢 Low.

---

## PR-B — refactor (14 tasks, ~400-500 líneas)

Branch: `feat/pre-wave-2-pr-b-refactor` (se crea desde `main` post-merge de PR-A).

### Task B.1 — Create `to_ipc_error` helper module

- Footprint: `src-tauri/src/ipc_error.rs` (nuevo, ~20 líneas + tests), `src-tauri/src/lib.rs` (declarar `mod ipc_error;`).
- Action: `pub(crate) fn to_ipc_error(e: AppError) -> String` que serializa vía `Serialize for AppError` (engine/src/lib.rs:82-150); 4-5 tests unitarios cubriendo `FileNotFound`, `AIUnavailable`, `Database`.
- Test: `cargo test` desde `src-tauri/` — `to_ipc_error` tests verdes.
- Spec: `IPC boundary emits structured IpcErrorPayload`.
- Risk: 🟢 Low.

### Task B.2 — Introduce `AIServicePort` trait

- Footprint: `engine/src/ai/service.rs:1-52` (extender).
- Action: `pub(crate) trait AIServicePort: Send + Sync` con 2 métodos async (signatura design D2 líneas 84-103); `impl<R: AIProviderResolver + Send + Sync> AIServicePort for AIService<R>`; renombrar `explain_node`/`chat` → `*_with_context`; tests con mock resolver.
- Test: `cargo test --lib ai::service` desde `engine/`.
- Spec: `AIService is consumed through AIServicePort`.
- Risk: 🟡 Medium (renombrar rompe call sites en commands.rs:240-398; ajustar atómicamente).

### Task B.3 — Add `from_arc` constructors to repository adapters

- Footprint: `engine/src/ports.rs:68-78, 162-172, 335-345, 533-545` (4 adapters).
- Action: en cada uno de `ScanRepositoryAdapter`, `GraphRepositoryAdapter`, `WorkspaceRepositoryAdapter`, `AnalysisRepositoryAdapter`: cambiar struct de `<'pool>` a `'static` con `Arc<DbPool>` interno; **preservar** `::new(&pool)` para tests internos; **agregar** `::from_arc(Arc<DbPool>)` (patrón de `AppStatePortAdapter::from_arc_refs:666-676`).
- Test: `cargo test` desde `engine/`; agregar 1 test por adapter que verifique que `from_arc` retorna un struct con pool vivo.
- Spec: `AppState holds Arc<dyn> ports` (composition root scenario).
- Risk: 🟡 Medium (cambio de struct shape).

### Task B.4 — AppState step 1: `Arc<dyn AIServicePort>`

- Footprint: `src-tauri/src/commands.rs:30-38, 240-317, 319-398`; `src-tauri/src/lib.rs:48-54`.
- Action: agregar `ai_service: Arc<dyn AIServicePort>` a `AppState`; marcar el viejo `ai_service: engine::ai::AIService:37` con `#[allow(dead_code)]` hasta B.9; construir en `lib.rs` con `Arc::new(engine::ai::AIService::default()) as Arc<dyn AIServicePort>`; reescribir `explain_node` y `chat` a 5-10 líneas que delegan (la mudanza completa del cuerpo va en B.10/B.11).
- Test: `cargo check` + `cargo test` desde `src-tauri/`.
- Spec: `AppState holds Arc<dyn> ports` + `AIService is consumed through AIServicePort`.
- Risk: 🟡 Medium (acoplado al shape de B.2).

### Task B.5 — AppState step 2: `Arc<dyn ScanRepository>`

- Footprint: `src-tauri/src/commands.rs:30-38, 61-125` (3 comandos); `src-tauri/src/lib.rs:48-54`.
- Action: agregar `scan_repo: Arc<dyn ScanRepository>`; en `lib.rs` agregar `Arc::new(ScanRepositoryAdapter::from_arc(pool.clone())) as Arc<dyn ScanRepository>`; reescribir 3 comandos (`scan_project`, `open_project_by_path`, `get_scan_status`) a 4-6 líneas con `state.scan_repo.clone()`.
- Test: `cargo test` desde `src-tauri/`.
- Spec: `AppState holds Arc<dyn> ports`.
- Risk: 🟢 Low.

### Task B.6 — AppState step 3: `Arc<dyn GraphRepository>`

- Footprint: `src-tauri/src/commands.rs:30-38, 140-218` (4 comandos); `src-tauri/src/lib.rs:48-54`.
- Action: análogo a B.5 con `graph_repo` y los 4 comandos (`get_graph`, `get_node_details`, `get_node_outline`, `search_nodes`).
- Test: `cargo test` desde `src-tauri/`.
- Spec: `AppState holds Arc<dyn> ports`.
- Risk: 🟢 Low.

### Task B.7 — AppState step 4: `Arc<dyn WorkspaceRepository>`

- Footprint: `src-tauri/src/commands.rs:30-38, 525-668` (~143 líneas, 13 comandos); `src-tauri/src/lib.rs:48-54`.
- Action: agregar `workspace_repo: Arc<dyn WorkspaceRepository>`; reescribir 13 comandos; ajustar macro `workspace_service!` para que reciba el port en vez de `&DbPool` (si la macro bloquea, expandir manualmente los 13 commands).
- Test: `cargo test` desde `src-tauri/`; verificar que la macro expande correctamente.
- Spec: `AppState holds Arc<dyn> ports`.
- Risk: 🔴 High (13 comandos + macro `workspace_service!` agregan variabilidad).

### Task B.8 — AppState step 5: `Arc<dyn AnalysisRepository>`

- Footprint: `src-tauri/src/commands.rs:30-38, 411-468` (4 comandos); `src-tauri/src/lib.rs:48-54`.
- Action: análogo a B.5 con `analysis_repo` y los 4 comandos (`get_architecture_detection`, `get_impact_analysis`, `get_graph_insights`, `export_view`).
- Test: `cargo test` desde `src-tauri/`.
- Spec: `AppState holds Arc<dyn> ports`.
- Risk: 🟢 Low.

### Task B.9 — AppState step 6: final cleanup

- Footprint: `src-tauri/src/commands.rs:30-38`; `src-tauri/src/lib.rs:48-54`.
- Action: eliminar `pub db: DbPool:31`; eliminar `ai_service: engine::ai::AIService:37` y el `#[allow(dead_code)]`; eliminar imports redundantes; verificar `rg "pub (db|ai_service):" src-tauri/src/commands.rs` → 0 hits. Los 3 `Arc<Mutex<...>>` (scan_status, ai_config, project_root) **se mantienen** como primitive collaborators (spec lo permite).
- Test: `cargo check` + `cargo test` desde `src-tauri/`; `rg "pub (db|ai_service):"` → 0 hits.
- Spec: `AppState holds Arc<dyn> ports` (fields all trait objects or primitives).
- Risk: 🟡 Medium (cambiar `db: DbPool` rompe cualquier call site residual no cubierto por B.5-B.8).

### Task B.10 — Move `explain_node` logic to `AIService::explain_node_with_context`

- Footprint: `src-tauri/src/commands.rs:240-317` (78 líneas) → `engine/src/ai/service.rs` (impl de `AIServicePort`).
- Action: mover el cuerpo (file read, `ProjectRepository::new(&state.db)` → `state.scan_repo.clone()`, `ContextBuilder::build_node_context`, edge filter) al método del trait; el command body queda en 5-10 líneas que delegan con `.map_err(to_ipc_error)`; agregar test con mock repository.
- Test: `cargo test --lib ai::service` desde `engine/`.
- Spec: `AIService is consumed through AIServicePort`.
- Risk: 🟡 Medium (78 líneas; clock leak menor con `Uuid` / `chrono::Utc` documentado).

### Task B.11 — Move `chat` logic to `AIService::chat_with_context`

- Footprint: `src-tauri/src/commands.rs:319-398` (80 líneas) → `engine/src/ai/service.rs` (impl de `AIServicePort`).
- Action: análogo a B.10; el command body queda en 5-10 líneas con `.map_err(to_ipc_error)`; test con mock.
- Test: `cargo test --lib ai::service` desde `engine/`.
- Spec: `AIService is consumed through AIServicePort`.
- Risk: 🟡 Medium.

### Task B.12 — Atomic error-contract rollout

- Footprint: `src-tauri/src/commands.rs` (37 ocurrencias de `.map_err(|e| e.to_string())`), `src/lib/tauri-api.ts:58-126`, `src/lib/__tests__/tauri-api.test.ts`, `src/hooks/__tests__/useAI-corrective.test.ts` (si existe).
- Action: en UN SOLO COMMIT — (1) backend: 37 reemplazos `.map_err(|e| e.to_string())` → `.map_err(to_ipc_error)`; mapear strings sueltos (`"AI not configured"`, `format!("File not found: ...", node_id)`) a `AppError::AIUnavailable` / `AppError::FileNotFound`; (2) frontend: `toApiError` ya parsea JSON; documentar rama legacy como defensiva; (3) tests: actualizar fixtures para reflejar el nuevo contrato.
- Test: `cargo test` + `npm run test` verdes simultáneamente; `rg "map_err\(\|e\| e\.to_string\(\)\)" src-tauri/src/commands.rs` → 0 hits; `npm run check:arch` verde.
- Spec: `IPC boundary emits structured IpcErrorPayload` + `Atomic rollout of error contract` (MODIFIED).
- Risk: 🔴 High (atomicidad exigida por spec; un commit que toca 4 grupos de archivos; disciplina de NO commitear hasta que ambos test suites estén verdes).

### Task B.13 — Delete `src/services/*.ts` and rename test file

- Footprint: `src/services/{ai,project,graph,snapshot,analysis}Service.ts` (5 archivos, 257 líneas, **borrar**); `src/services/__tests__/services-boundary.test.ts` (430 líneas, **renombrar**); 9 hooks + 1 store (10 imports, **migrar**); `src/lib/__tests__/tauri-api-bridge.test.ts` (nuevo, ~280 líneas).
- Action: 3 sub-pasos en 3 commits — (1) `git mv` test file; reducir a 1 parser test + 1 smoke test + 1 static-guard; (2) migrar 10 imports en 9 archivos a `@/lib/tauri-api` (lista completa: `useAI.ts:7`, `useAIConfig.ts:5`, `useGraph.ts:5`, `useNodeDetails.ts:5`, `useNodeOutline.ts:5`, `useProject.ts:11,12`, `useArchitecture.ts:9`, `useExport.ts:5`, `useSnapshotStore.ts:7`); (3) `git rm` los 5 archivos `.ts` y `git rm -r src/services/`. Extender `npm run check:arch` con pattern `rg "from ['\"](@/|\.\./)services" src/` → 0 hits.
- Test: `find src/services -name "*.test.ts"` → 0; `rg "from ['\"](@/|\.\./)services" src/` → 0; `npm run test` + `npm run typecheck` + `npm run lint` verdes.
- Spec: `hooks consume tauri-api directly` (ADDED) + `services-boundary test becomes tauri-api bridge test` (ADDED) + `Frontend domain services wrap the Tauri bridge` (REMOVED).
- Risk: 🟡 Medium (3 commits pequeños secuenciales; cada uno debe quedar verde antes del siguiente).

### Task B.14 — PR-B final verification

- Action: full quality gates; `wc -l src-tauri/src/commands.rs` ≤ 350; `ls src-tauri/src/ipc_error.rs` existe; `rg "pub (db|ai_service):" src-tauri/src/commands.rs` → 0; `rg "scan_repo|graph_repo|workspace_repo|analysis_repo|ai_service|app_state" src-tauri/src/commands.rs | rg "Arc<dyn"` muestra exactamente 6 `Arc<dyn ...>` fields; `rg "use engine::ports::(Scan|Graph|Workspace|Analysis)Repository\b" src-tauri/` → 0; `rg "from ['\"](@/|\.\./)services" src/` → 0; `rg "map_err\(\|e\| e\.to_string\(\)\)" src-tauri/src/commands.rs` → 0; `git diff --stat main..HEAD` ≤ 500 líneas.
- Acceptance: todos los gates verdes; diff ≤ 500; listo para abrir PR encadenado.
- Spec: criteria PR-B (proposal:89-96) + wave-2 readiness (proposal:98-101).
- Risk: 🟢 Low.

---

## Dependency Graph

```
PR-0 (DONE)  →  PR-A: A.1, A.2, A.3, A.4  →  A.5
PR-A merged  →  PR-B: B.1, B.2, B.3
              →  (B.4  ||  B.5  ||  B.6  ||  B.7  ||  B.8)
              →  B.9
              →  (B.10  ||  B.11  ||  B.12  ||  B.13)
              →  B.14
```

B.4-B.8 son 5 pasos paralelos (recomendado: secuencial para review). B.10/B.11 dependen de B.4. B.12 puede ir en cualquier momento post-B.1, recomendado commit dedicado al final. B.13 independiente del backend.

---

## Top Risks (resumen)

- 🔴 B.7 — 13 comandos de workspace + macro `workspace_service!` agregan variabilidad.
- 🔴 B.12 — atomicidad backend + frontend + tests exigida por spec; disciplina de no commitear hasta `cargo test` Y `npm run test` verdes simultáneamente.
- 🟡 B.2 — renombrar `explain_node`/`chat` rompe call sites; ajustar atómicamente.
- 🟡 A.1 — escape hatch del integration test `workspace_service_test.rs:20`.
