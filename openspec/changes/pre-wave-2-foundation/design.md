# Diseño: pre-wave-2-foundation

## Enfoque técnico

Este change ejecuta una **fundación quirúrgica en dos PRs encadenados** que convierte los compromisos de los specs (`error-contract`, `backend-ports-and-services`, `frontend-service-layer`) en constraints verificables a nivel de compilación y de CI, sin tocar wave 2 (split de crates, CQRS, events). PR-A instala los cimientos sin cambios de signatura: `pub(crate)` en los 5 port traits, un guard JS que bloquea regresiones, dedup de helpers de error duplicados, y el docstring del contrato `Arc<Mutex<...>>`. PR-B ejecuta la mudanza: helpers de IA en `AIService` con su port trait delgado, `AppState` con `Arc<dyn ...>`, helper `to_ipc_error` que honra el envelope JSON, y borrado de la capa `src/services/*.ts` del frontend.

La frontera del crate `engine` queda **sellada por compilación y por CI** después de los dos PRs. Wave 2 puede partir el crate sabiendo que ningún módulo de presentación está acoplado a infraestructura concreta.

## Decisiones de arquitectura

### Decisión 1: ubicación y forma de `to_ipc_error`

**Elección**: **(a) archivo nuevo `src-tauri/src/ipc_error.rs`** — `pub(crate) fn to_ipc_error(e: AppError) -> String`, sin genéricos, sin `IpcErrorPayload` intermedio.

**Rationale**:
- `src-tauri/src/commands.rs` ya tiene 669 líneas (línea 669) y es el archivo más caliente del change; añadir 30-50 líneas más lo lleva a 720+ y rompe el objetivo de "≤350 líneas" del success criterion PR-B (`proposal.md:100`).
- `engine/src/lib.rs:74-80` ya define `IpcErrorPayload` y la impl `Serialize for AppError` (líneas 82-150) — producir un `IpcErrorPayload` intermedio y serializarlo a `String` desde la presentación sería doble serialización (ya hay un `serde::Serialize` que emite el payload); basta con un `to_string()` de un value que implemente `Serialize` a `serde_json::to_string`.
- `engine/src/lib.rs` (crate `engine`) no es el lugar correcto porque el helper **es policy de la capa de presentación** (cómo se traduce `AppError` a un mensaje que cruce IPC), no de dominio.

**Alternativas consideradas**:
- (b) Top of `commands.rs`: rechazado, infla el archivo que estamos adelgazando.
- (c) En `lib.rs` junto a `run()`: rechazado, `lib.rs` es composition root; mezclar policy de transporte con bootstrap es ruido.
- (d) En `engine/src/lib.rs` como `pub fn`: rechazado, fuerza a la presentación a depender de un util de presentation-shaped export del crate `engine`.

**Evidencia**:
- `engine/src/lib.rs:82-150` — la impl `Serialize for AppError` produce exactamente el `IpcErrorPayload` JSON que el parser frontend espera; reusarla es la implementación trivial.
- `src-tauri/src/commands.rs:71, 93, 109, ..., 668` — 37 ocurrencias de `.map_err(|e| e.to_string())` (verificado en grep), todas candidatas a reemplazo mecánico por `.map_err(to_ipc_error)`.
- `src/lib/tauri-api.ts:58-126` — el parser `toApiError` ya parsea JSON estructurado y la rama legacy es código muerto después de este change.

**Firma y cuerpo**:

```rust
// src-tauri/src/ipc_error.rs
use engine::AppError;

/// Serializa un AppError a un JSON string estructurado en forma de IpcErrorPayload.
///
/// Único punto de conversion desde AppError a wire format. El frontend parsea
/// este JSON en `toApiError` (src/lib/tauri-api.ts:58-126) para construir un
/// ApiError tipado. NO emitir strings sueltos: la rama legacy del parser es
/// código muerto y se conserva solo defensivamente durante el rollout window.
pub(crate) fn to_ipc_error(e: AppError) -> String {
    // Reusa la impl Serialize existente en engine::lib (lib.rs:82-150) que
    // produce el IpcErrorPayload canónico.
    serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
}
```

**Visibilidad**: `pub(crate)` — la presentación es la única consumidora, ningún test externo la necesita.

**Genéricos**: NO genérico sobre `R: Serialize`. La función es monomórfica sobre `AppError`; ese es el contrato. Generalizar agregaría superficie sin valor.

### Decisión 2: shape del trait `AIServicePort`

**Elección**: **`pub(crate) trait AIServicePort: Send + Sync`** en `engine/src/ai/service.rs` (mismo archivo que `AIService`), 2 métodos async, `AIService<R: AIProviderResolver>` implementa el trait trivialmente.

**Rationale**:
- El spec `backend-ports-and-services/spec.md:97-115` exige el trait estrecho, exactamente 2 métodos, en `engine::ai::service::AIServicePort`.
- Crear `engine/src/ai/port.rs` separado es separación ceremonial sin valor: el trait está lógicamente acoplado a `AIService` (su única implementadora) y vive en su mismo módulo.
- `Send + Sync` es **obligatorio** porque `AppState` guarda `Arc<dyn AIServicePort>` (lo exige `tauri::State<...>` que es `Send + Sync`); sin bounds, el trait no compila en posición de field.
- Los tipos de los métodos (`AIConfig`, `FileInfo`, `GraphData`, `Vec<ChatMessage>`) ya son re-exports públicos del crate `engine` en `lib.rs:16-18`, así que el trait puede usarlos en su signatura pública-a-crate sin exponer más superficie.

**Alternativas consideradas**:
- Trait sin `Send + Sync`: rechazado, `Arc<dyn ...>` en `AppState` requiere el bound; cualquier test E2E multi-threaded fallaría.
- Trait con `pub` (no `pub(crate)`): rechazado, el spec exige `pub(crate)` para mantenerlo como seam interno; los adapters concretos (`AnthropicProvider`, `ResolvedProvider`, `ProviderFactory`) son la verdadera "public AI API" y ya están ocultos por la visibility del módulo `ai`.
- Trait con 3+ métodos (agregando `configure_ai`, `get_ai_config`): rechazado, esos viven en `AppStatePort` y son concerns de estado, no de orquestación de IA.
- Use cases separados (`ExplainNodeUseCase`, `ChatUseCase`): rechazado, el spec explícitamente dice "thin trait con 2 métodos" (`backend-ports-and-services/spec.md:114`).

**Evidencia**:
- `engine/src/ai/service.rs:8-24` — `AIService<R = ProviderFactory>` ya es la implementación canónica; agregar el trait al lado es ~25 líneas.
- `engine/src/ai/service.rs:30-51` — los 2 métodos existentes (`explain_node`, `chat`) son las semillas exactas de los 2 métodos del port trait; se renombran a `explain_node_with_context` y `chat_with_context` para reflejar que reciben contexto pre-construido (el orquestador de contexto vive en la presentación / en un método wrapper del propio service).
- `engine/src/ai/service.rs:54-181` — los tests existentes con `TestProvider`/`TestResolver` siguen aplicando al impl; se agregan tests específicos para los 2 métodos del port trait con un mock repository que provea `FileInfo`, `GraphData`, etc.

**Shape del trait**:

```rust
// en engine/src/ai/service.rs (extensión, no archivo nuevo)
use crate::models::{AIConfig, ChatMessage, ChatResponse, FileInfo, GraphData, NodeExplanation};

/// Port trait para AIService — la presentación consume IA solo a través de este trait.
///
/// Send + Sync es obligatorio para Arc<dyn AIServicePort> en AppState.
pub(crate) trait AIServicePort: Send + Sync {
    async fn explain_node_with_context(
        &self,
        config: &AIConfig,
        file_info: &FileInfo,
        file_content: &str,
        graph: &GraphData,
        outline: &[crate::models::OutlineItem],
    ) -> crate::Result<NodeExplanation>;

    async fn chat_with_context(
        &self,
        config: &AIConfig,
        project_id: &str,
        root_path: &str,
        file_contents: &[(String, String)],
        graph: &GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> crate::Result<ChatResponse>;
}

impl<R: AIProviderResolver + Send + Sync> AIServicePort for AIService<R> {
    // implementación: orquesta ContextBuilder internamente, ya no en commands.rs
}
```

### Decisión 3: constructores `from_arc` de los 4 adapters de repository + `AIServicePortAdapter`

**Elección**: agregar **`Adapter::from_arc(Arc<DbPool>) -> Self`** a cada uno de los 4 adapters, **manteniendo** `Adapter::new(&DbPool)` para los tests internos del crate.

**Rationale**:
- `explore.md:148` recomienda esta opción explícitamente para preservar compatibilidad con los 5 tests de `engine/src/services/scan_service.rs:380, 410, 433, 468, 487` que usan `Adapter::new(&pool)`.
- Los adapters actuales son `<'pool>` (lifetime en `ScanRepositoryAdapter<'pool>`, `GraphRepositoryAdapter<'pool>`, `WorkspaceRepositoryAdapter<'pool>`, `AnalysisRepositoryAdapter<'pool>`) — esto bloquea guardarlos en `Arc<dyn ...>` directamente. La solución es **eliminar el lifetime y almacenar `Arc<DbPool>` internamente**, pero eso rompe los tests.
- La salida: `from_arc(Arc<DbPool>)` retorna un nuevo struct `'static` que contiene un `Arc<DbPool>` (clonado), y `new(&DbPool)` se mantiene para tests. Tests existentes siguen funcionando; `AppState` puede construir desde el `Arc` clonado en `lib.rs:48-54`.

**Alternativas consideradas**:
- `From<Arc<DbPool>> for Adapter` en vez de `from_arc`: rechazado, no permite coexistencia con `From<&DbPool>` (trait coherence rules en Rust).
- Eliminar `new(&DbPool)` y migrar los 5 tests: rechazado, infla el diff de PR-B ~80 líneas más (5 tests × ~15 líneas de cambio), excede budget.
- Hacer que `AppState` guarde `Arc<DbPool>` y los adapters se construyan bajo demanda en cada comando: rechazado, derrotamos la optimización del trait object y agregamos 4 clones por comando.

**Evidencia**:
- `engine/src/ports.rs:73, 167, 340, 539` — los 4 constructores `::new(pool: &'pool DbPool)` que reciben referencia.
- `engine/src/services/scan_service.rs:380, 410, 433, 468, 487` — los 5 call sites de tests que usan `Adapter::new(&pool)` (mismo patrón en otros services).
- `engine/src/ports.rs:666-676` — `AppStatePortAdapter::from_arc_refs` ya existe y es el patrón a seguir; se documenta explícitamente que la versión `&Arc<Mutex<T>>` es porque los campos del `AppState` son `Arc<Mutex<...>>` (compartidos con la presentación).

**Migración de tests**:
- Los 5 tests de `scan_service.rs` que usan `Adapter::new(&pool)` → **sin cambios** (se preserva la API).
- Cualquier test nuevo que quiera el nuevo path usa `Adapter::from_arc(Arc::new(pool))` o, mejor, usa un mock repository (más alineado con la arquitectura target).

**Signaturas nuevas** (las 4, mismo patrón):

```rust
// en engine/src/ports.rs — ejemplo para ScanRepositoryAdapter
pub struct ScanRepositoryAdapter {
    inner: crate::db::queries::ProjectRepository<'static>,
    _pool: Arc<DbPool>, // mantiene el Arc vivo
}

impl ScanRepositoryAdapter {
    pub fn new(pool: &DbPool) -> Self { /* lifetime corto, para tests */ }
    pub fn from_arc(pool: Arc<DbPool>) -> Self {
        Self {
            inner: crate::db::queries::ProjectRepository::new(&pool),
            _pool: pool,
        }
    }
}
```

Nota: `ProjectRepository<'pool>` requiere un lifetime; la solución es usar `'static` con el `Arc<DbPool>` clonable. Análisis análogo aplica a los otros 3 adapters.

### Decisión 4: shape del script CI guard

**Elección**: `scripts/ci/check-architecture.mjs` (Node, CommonJS modules), invocado por `"check:arch": "node scripts/ci/check-architecture.mjs"` en `package.json:6-16`. **Política de `arch-allow`**: **estricta, los comentarios NO se honran** (el spec `backend-ports-and-services/spec.md:86-89` lo dice explícitamente).

**Rationale**:
- `openspec/config.yaml:12-13` y `proposal.md:Q3` confirman `npm run check:arch` como decisión cerrada.
- Node (no bash): reutiliza el toolchain ya instalado (`.github/workflows/ci.yml:21-24` cachea npm), evita dep nueva.
- Política estricta: el spec lo exige textualmente. Conceder excepciones por comentario es un anti-pattern (se acumulan sin auditoría). El path de excepción es **PR con label `arch-exception`** y justificación, fuera del script.
- Exit code 1 ante cualquier violación; imprime TODAS las violaciones (no se detiene en la primera) para que el dev arregle en una pasada.

**Forbidden patterns** (regex, file:line de la decisión):

| Pattern regex | Archivo target | Razón |
|---|---|---|
| `use engine::db::` | `src-tauri/src/commands.rs` | Acopla presentación a SQLite concreto |
| `use engine::ai::anthropic` | `src-tauri/src/commands.rs` | Bypassa el port, expone provider concreto |
| `use engine::ai::resolved` | `src-tauri/src/commands.rs` | Idem |
| `use engine::ai::provider::AIProvider` | `src-tauri/src/commands.rs` | Idem |
| `use engine::ai::AIService` | `src-tauri/src/commands.rs` | Después de PR-B, solo `AIServicePort` |
| `\.map_err\(\|e\| e\.to_string\(\)\)` | `src-tauri/src/commands.rs` | Después de PR-B, debe ser `to_ipc_error` |

**Self-test**: el script incluye una fixture `scripts/ci/__fixtures__/forbidden.rs` con 6 líneas (1 por pattern) y una fixture `scripts/ci/__fixtures__/clean.rs` con código válido. El script corre contra ambas en un sub-comando `--self-test` y verifica que la fixture prohibida falla y la limpia pasa. Esto se ejecuta en `npm run check:arch -- --self-test` y en CI.

**Wiring en CI**: añadir un step **dentro del job `rust-backend` existente** (no un job nuevo):

```yaml
      - name: Architecture guard
        run: npm run check:arch
```

Razón: corre en el mismo runner que `cargo test` y comparte el checkout; no necesita un setup-node duplicado. La `lint-and-typecheck` job es para el frontend; el guard es de la frontera backend-frontend así que pertenece a `rust-backend`.

**Header del script** (los primeros 20 líneas, documentan la política):

```js
#!/usr/bin/env node
/**
 * check-architecture.mjs — CI guard para la frontera hexagonal.
 *
 * Politica: ESTRICTA. NO se honran comentarios // arch-allow.
 * Para pedir una excepcion, abrir PR con label `arch-exception` y
 * justificacion documentada en el cuerpo del PR.
 *
 * Patterns prohibidos — ver specs/backend-ports-and-services/spec.md.
 * Exit code 0 = clean, 1 = alguna violacion.
 */
```

**Estrategia de revisión**: el dev corre `npm run check:arch` localmente antes de pushear. CI falla si se cuela algo. El `lint-staged` hook (`.husky/`, `package.json:54-63`) **no** se modifica para invocar el guard — sería lento en cada commit; el guard es responsabilidad de `pre-push` o del CI.

### Decisión 5: impacto de `pub(crate)` en los 5 port traits

**Elección**: cambiar las 5 declaraciones de trait en `engine/src/ports.rs`:
- Línea 37: `pub trait ScanRepository` → `pub(crate) trait ScanRepository`
- Línea 139: `pub trait GraphRepository` → `pub(crate) trait GraphRepository`
- Línea 224: `pub trait WorkspaceRepository` → `pub(crate) trait WorkspaceRepository`
- Línea 499: `pub trait AnalysisRepository` → `pub(crate) trait AnalysisRepository`
- Línea 607: `pub trait AppStatePort` → `pub(crate) trait AppStatePort`

**Los adapters (`*Adapter`) NO cambian de visibilidad** — siguen siendo `pub` (líneas 68, 162, 335, 533, 636) porque la presentación los construye directamente desde `lib.rs:48-54` en PR-B.

**Verificación pre-apply** (los siguientes `rg` deben dar 0 hits):

```bash
rg "use engine::ports::ScanRepository"      src-tauri/
rg "use engine::ports::GraphRepository"     src-tauri/
rg "use engine::ports::WorkspaceRepository" src-tauri/
rg "use engine::ports::AnalysisRepository"  src-tauri/
rg "use engine::ports::AppStatePort"        src-tauri/
```

Resultado verificado: 0 hits en `src-tauri/`. El único `use engine::ports::` en `src-tauri/` es `commands.rs:510` (`use engine::ports::WorkspaceRepositoryAdapter` — el adapter, no el trait).

**Verificación de tests internos del crate** (deben seguir compilando porque son `crate::ports::...` y no `use engine::...`):

- `engine/tests/ports_test.rs:29, 38, 47, 56, 70, 115, 132, 151, 176, 227` — `use engine::ports::ScanRepositoryAdapter` y similares; **usan adapters, no traits**, así que no se rompen.
- `engine/tests/workspace_service_test.rs:20` — `use engine::ports::WorkspaceRepository;` ← **PROBLEMA**: este test importa el trait directamente. Está en `engine/tests/`, que es un test de integración considerado "externo" al crate `lib` y por lo tanto **sí se rompe** con `pub(crate)`.

**Escape hatch identificado**: `engine/tests/workspace_service_test.rs:20` necesita una solución. Opciones:
- **(a) Cambiar el test a `engine/src/services/workspace_service.rs` tests module** (`#[cfg(test)] mod tests` dentro del crate `lib`) — el trait sigue siendo visible. Costo: ~10-20 líneas de movimiento de test, y se pierde el formato "integration test".
- **(b) Hacer `WorkspaceRepository` `pub(super)` en vez de `pub(crate)`** — sigue siendo privado al crate `engine` desde la perspectiva del binario `src-tauri`, pero los tests de integración en `engine/tests/` están en el crate `engine` por construcción (Cargo compila `tests/*.rs` como crates separados que **vinculan al crate `engine` como dependencia externa**). **Espera**: `pub(crate)` significa "visible dentro del crate actual" — y los integration tests son crates separados, así que `pub(crate)` no los incluye. Confirmar con `cargo`: lo más probable es que `engine/tests/*.rs` **se rompa** porque la regla de visibility se evalúa desde la perspectiva del crate consumidor.

**Decisión final para el escape hatch**: **mover los 3-4 tests de `engine/tests/workspace_service_test.rs` que importan el trait `WorkspaceRepository` al módulo `#[cfg(test)] mod tests` de `engine/src/services/workspace_service.rs`**. Esto preserva `pub(crate)` (el objetivo del change) y mantiene los tests corriendo. Estimación: ~30 líneas movidas, dentro del budget de PR-A.

**Verificación exhaustiva de tests** (los siguientes `rg` deben dar 0 hits o hits solo en `engine/src/`):

```bash
rg "use engine::ports::(Scan|Graph|Workspace|Analysis)Repository\b" engine/
rg "use engine::ports::AppStatePort\b" engine/
```

Cualquier hit en `engine/tests/*.rs` es candidato a mover.

### Decisión 6: estrategia de dedup de error-mapping

**Elección**: **(d) cambio de firma del servicio a `Result<T, ScanError>` con `From<ScanError> for AppError`** — los comandos usan `?` y el helper de error-mapping se mantiene como detalle interno del servicio.

**Rationale**:
- `engine/src/services/scan_service.rs:325-336` ya tiene la versión "typed" (`map_save_scan_result_error` retorna `AppError` directamente). La versión duplicada en `commands.rs:490-501` retorna `String` y se quedó como residuo pre-wave-1.
- La opción (a) — hacer `pub` los helpers en `ScanService` — fuerza un cross-layer call: presentación llama a un método del servicio para clasificar errores que el servicio ya conoce. Es una dirección de dependencia invertida fea.
- La opción (b) — módulo `db::error_mapping` — agrega un módulo nuevo para 17 líneas de lógica que viven perfectamente en `scan_service.rs`.
- La opción (c) — método `classify_error` público en `ScanService` — equivalente a (a) con otro nombre.
- La opción (d) elimina la necesidad de un helper externo: el servicio ya sabe cuándo es `UNIQUE constraint failed: projects.root_path` (líneas 171-181, 241-251) y retorna el `AppError` correcto. Los comandos nunca ven el string; reciben un `AppError` y el helper `to_ipc_error` lo serializa.

**Alternativa considerada** (sub-óptima): eliminar las funciones de `commands.rs` y hacer que `scan_service::is_root_path_conflict` y `map_save_scan_result_error` sean `pub(crate)` para que los tests de `observability_tests.rs` los puedan invocar. **Rechazada** porque expone detalles de SQLite a través del crate, y el `pub(crate)` no resuelve la duplicación conceptual.

**Migración de los 7 tests de `observability_tests.rs:1-83`**: los tests verifican comportamiento del clasificador, no de la API pública. Se renombran y se mueven a `engine/src/services/scan_service.rs` como métodos `#[cfg(test)]` adicionales en el `mod tests` existente (líneas 338-495):

```rust
// en engine/src/services/scan_service.rs, mod tests

#[test]
fn is_root_path_conflict_true_for_root_path() {
    assert!(is_root_path_conflict("UNIQUE constraint failed: projects.root_path"));
}
// ... 6 tests más
```

Costo: ~60 líneas de test movidas + ~5 líneas de import. Beneficio: los tests quedan junto a la lógica que verifican.

**Backward compat**: **NO** se mantiene ninguna función en `commands.rs` con esos nombres. El único consumidor fuera del módulo `tests` es el propio módulo `tests` (que estamos moviendo), así que no hay backward compat que preservar. Los 7 tests renombrados pasan a ser tests internos del servicio.

**Sutil cambio detectado en `explore.md:107`**: el `map_save_scan_result_error` original en `commands.rs:497` retornaba `"Project already exists at path: ..."` (un mensaje user-facing), mientras que el de `scan_service.rs:332` retorna `AppError::ProjectNotFound(root_path)` (que serializa a `PROJECT_NOT_FOUND` con `details: { path }`). **El comportamiento frontend cambia**: el parser `toApiError` mapea `PROJECT_NOT_FOUND` a `PATH_NOT_FOUND` (`tauri-api.ts:36`), así que el error user-facing va a ser "No se encontró el archivo o proyecto solicitado" (de `toUserMessage` línea 144-145). **Esto es correcto y deseado**: el contrato de error unificado se honra, y el frontend ya tiene cobertura para `PATH_NOT_FOUND`.

### Decisión 7: secuencia de refactor de `AppState`

**Elección**: **6 pasos incrementales, uno por port, en PR-B**, NO big-bang.

**Rationale**:
- Big-bang: 22 comandos × 4 líneas promedio = 88 líneas tocadas + 5 signaturas de service + 4 constructores + composition root. ~300 líneas en un solo commit, atómicas pero difíciles de revisar.
- Incremental: cada paso toca 1-4 comandos y 1 service; el reviewer puede verificar cada paso independientemente. Riesgo de regresión acotado por paso.
- Costo: el PR-B completo es ~500 líneas (justifica chained PR), pero **dividido en 6 commits lógicos** dentro del PR, cada uno commiteable y revertible.

**Secuencia propuesta (dentro de PR-B, 6 commits)**:

1. **AI port**: introduce `AIServicePort` trait en `engine/src/ai/service.rs`; agrega `ai_service_port: Arc<dyn AIServicePort>` en `AppState`; refactoriza `explain_node` (líneas 240-317) y `chat` (líneas 319-398) para delegar al port via `state.ai_service_port`. Costo: ~180 líneas. El campo `ai_service: AIService` se mantiene (dead, marcado `#[allow(dead_code)]`) hasta paso 6.

2. **Scan port**: introduce `Adapter::from_arc` en `ScanRepositoryAdapter`; agrega `scan_repo: Arc<dyn ScanRepository>` en `AppState`; refactoriza 3 comandos (líneas 61-72, 80-94, 101-125) y `ScanService::new` para tomar `Arc<dyn ScanRepository>`. Costo: ~40 líneas. Elimina 3 ocurrencias de `&state.db`.

3. **Graph port**: análogo a Scan, refactoriza 4 comandos (líneas 140-153, 159-171, 180-195, 201-218) y `GraphService::new`. Costo: ~50 líneas.

4. **Workspace port**: análogo, refactoriza los 13 comandos de `workspace_service!` macro (líneas 525-668) y `WorkspaceService::new`. Costo: ~50 líneas.

5. **Analysis port**: análogo, refactoriza 4 comandos (líneas 411-468) y `AnalysisService::new`. Costo: ~40 líneas.

6. **Limpieza final**: introduce `app_state_port: Arc<dyn AppStatePort>` para que el campo `Arc<Mutex<...>>` también quede detrás del port; remueve los campos `db: DbPool`, `ai_service: AIService`, y todos los `Arc<Mutex<...>>` que ahora viven en `AppStatePortAdapter::from_arc_refs`; actualiza `lib.rs:48-54` para construir los 6 `Arc<dyn ...>`. Costo: ~80 líneas.

**Patrón de cambio por comando** (todos los 22 siguen el mismo template):

```rust
// ANTES (ejemplo línea 61-72):
let scan_repo = ScanRepositoryAdapter::new(&state.db);
let app_state_adapter = AppStatePortAdapter::from_arc_refs(&state.scan_status, &state.ai_config, &state.project_root);
let service = ScanService::new(scan_repo, app_state_adapter);
service.scan_project(&path).map_err(|e| e.to_string())

// DESPUÉS (paso 2):
let service = ScanService::new(state.scan_repo.clone(), state.app_state.clone());
service.scan_project(&path).map_err(to_ipc_error)
```

**Composition root nuevo** (`src-tauri/src/lib.rs:48-54` después de PR-B):

```rust
let pool = Arc::new(db_pool); // db_pool: engine::db::DbPool
let app_state = AppState {
    scan_repo: Arc::new(ScanRepositoryAdapter::from_arc(pool.clone())),
    graph_repo: Arc::new(GraphRepositoryAdapter::from_arc(pool.clone())),
    workspace_repo: Arc::new(WorkspaceRepositoryAdapter::from_arc(pool.clone())),
    analysis_repo: Arc::new(AnalysisRepositoryAdapter::from_arc(pool.clone())),
    ai_service: Arc::new(engine::ai::AIService::default()) as Arc<dyn AIServicePort>,
    app_state: AppStatePortAdapter::from_arc_refs(
        Arc::new(Mutex::new(ScanStatus::Idle)),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(String::new())),
    ),
};
```

(El `pool` se mantiene vivo en `app_state` indirectamente vía los 4 adapters que lo retienen; el `db: DbPool` field se elimina.)

### Decisión 8: secuencia de migración frontend `src/services/*.ts`

**Elección**: **3 pasos incrementales, NO big-bang**, alineado con el spec `frontend-service-layer/spec.md:11`.

**Rationale**:
- El spec exige que `services-boundary.test.ts` se renombre a `tauri-api-bridge.test.ts` antes de que se borren los archivos `src/services/*.ts`. Si los borramos primero, los imports de los tests quedan dangling.
- Cada hook migrado es un commit limpio y revisable: la diff es "−1 import, +1 import" en una sola línea.
- Borrar los 5 archivos en un commit dedicado asegura que el borrado es atómico y el `rg "from.*services/"` post-condición se cumple en un solo commit verificable.

**Secuencia propuesta (3 commits dentro de PR-B)**:

1. **Renombrar test**: `git mv src/services/__tests__/services-boundary.test.ts src/lib/__tests__/tauri-api-bridge.test.ts`; actualizar el `describe` interno de "Service contracts" / "Hook contracts" a "tauri-api bridge"; agregar el test de static-guard "no module imports from deleted `src/services/*` path" usando `fs.readdirSync` + `grep`. Costo: ~150 líneas reducidas (de 430 a ~280).
2. **Migrar hooks** (9 imports, 9 archivos `src/hooks/*.ts` + `src/stores/useSnapshotStore.ts`):

| Archivo | Línea actual | Nueva línea |
|---|---|---|
| `src/hooks/useAI.ts:7` | `from '../services/aiService'` | `from '@/lib/tauri-api'` |
| `src/hooks/useAIConfig.ts:5` | `from '../services/aiService'` | `from '@/lib/tauri-api'` |
| `src/hooks/useGraph.ts:5` | `from '../services/graphService'` | `from '@/lib/tauri-api'` |
| `src/hooks/useNodeDetails.ts:5` | `from '../services/graphService'` | `from '@/lib/tauri-api'` |
| `src/hooks/useNodeOutline.ts:5` | `from '../services/graphService'` | `from '@/lib/tauri-api'` |
| `src/hooks/useProject.ts:11-12` | `from '../services/projectService'` + `from '../services/graphService'` | `from '@/lib/tauri-api'` (×2) |
| `src/hooks/useArchitecture.ts:9` | `from '../services/projectService'` | `from '@/lib/tauri-api'` |
| `src/hooks/useExport.ts:5` | `from '../services/analysisService'` | `from '@/lib/tauri-api'` |
| `src/stores/useSnapshotStore.ts:7` | `from '../services/snapshotService'` | `from '@/lib/tauri-api'` |

Costo: 1 commit, 9 archivos, 1 línea cada uno.

3. **Borrar `src/services/`**: `git rm src/services/{ai,project,graph,snapshot,analysis}Service.ts` (5 archivos, 257 líneas) y `git rm src/services/__tests__/` (directorio ya vacío post paso 1). Commit único con mensaje "Remove passthrough frontend service layer".

**Verificación post-condición** (corre en CI, agregada al job `lint-and-typecheck`):

```bash
rg "from ['\"]@?\.?\.?/services" src/ || (echo "Forbidden: src/services imports remain" && exit 1)
```

**Decisión sobre el alias `@/services` en `tsconfig.json`**: no se elimina del tsconfig (no es necesario, no genera errores). El spec `frontend-service-layer/spec.md:21-24` exige que el alias "no se use en código de runtime", lo cual se cumple por la ausencia de imports — el alias puede seguir existiendo como string en tsconfig sin daño.

## Flujo de datos

### AppState después de PR-B (shape final)

```
┌─────────────────────────────────────────────────────────────────┐
│ AppState (src-tauri/src/commands.rs)                            │
│                                                                 │
│  scan_repo:      Arc<dyn ScanRepository>                        │
│  graph_repo:     Arc<dyn GraphRepository>                       │
│  workspace_repo: Arc<dyn WorkspaceRepository>                   │
│  analysis_repo:  Arc<dyn AnalysisRepository>                    │
│  ai_service:     Arc<dyn AIServicePort>                         │
│  app_state:      AppStatePortAdapter                            │
│                  (owns Arc<Mutex<ScanStatus>>,                  │
│                   Arc<Mutex<Option<AIConfig>>>,                 │
│                   Arc<Mutex<String>>)                           │
└─────────────────────────────────────────────────────────────────┘
            │                │              │              │
            ▼                ▼              ▼              ▼
   ┌────────────────┐ ┌──────────────┐ ┌──────────┐ ┌──────────┐
   │ ScanService    │ │ GraphService │ │ WspSvc   │ │ AIService│
   │ GraphService   │ │ AnalysisSvc  │ │          │ │ (impl    │
   │ (comparten Arc)│ │              │ │          │ │  port)   │
   └────────────────┘ └──────────────┘ └──────────┘ └──────────┘
            │                │              │              │
            ▼                ▼              ▼              ▼
   ┌─────────────────────────────────────────────────────────────┐
   │  ScanRepositoryAdapter, GraphRepositoryAdapter, etc.        │
   │  — cada uno contiene Arc<DbPool> internamente (clonado)     │
   └─────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
                        Arc<engine::db::DbPool> (única instancia)
```

### Error flow (después de PR-B, `to_ipc_error` en su lugar)

```
  Service::method()  ──returns──▶  Result<T, AppError>
                                          │
                                          ▼
  Tauri command body:  service.foo().map_err(to_ipc_error)
                                          │
                                          ▼
  to_ipc_error(e) — usa impl Serialize for AppError (engine::lib.rs:82-150)
                                          │
                                          ▼
  String = '{"code":"FILE_NOT_FOUND","message":"...","details":{"path":"..."}}'
                                          │
                                          ▼  IPC Tauri (string transport)
                                          │
  src/lib/tauri-api.ts:65-89 — toApiError parsea JSON, mapea code, retorna ApiError tipado
                                          │
                                          ▼
  Hook: const err = toApiError(e); const msg = toUserMessage(err);
                                          │
                                          ▼
  Componente: muestra "No se encontró el archivo solicitado"
```

## Cambios de archivos

### PR-A (foundations)

| Archivo | Acción | Descripción |
|---|---|---|
| `engine/src/ports.rs` | Modify | 5 traits `pub` → `pub(crate)` (líneas 37, 139, 224, 499, 607) |
| `engine/tests/workspace_service_test.rs` | Modify o Move | 3-4 tests que importan `WorkspaceRepository` se mueven a `engine/src/services/workspace_service.rs::tests` |
| `scripts/ci/check-architecture.mjs` | Create | Script guard (~80 líneas) con self-test |
| `scripts/ci/__fixtures__/{forbidden,clean}.rs` | Create | Fixtures de self-test |
| `package.json:6-16` | Modify | Agregar `"check:arch": "node scripts/ci/check-architecture.mjs"` |
| `.github/workflows/ci.yml:35-56` | Modify | Agregar step "Architecture guard" en job `rust-backend` |
| `src-tauri/src/commands.rs:477-501` | Delete | Remover `is_root_path_conflict` y `map_save_scan_result_error` |
| `src-tauri/src/commands/tests/observability_tests.rs:1-83` | Delete | Migrar 7 tests a `engine/src/services/scan_service.rs` |
| `engine/src/services/scan_service.rs:320-336` | Modify | Hacer `is_root_path_conflict` y `map_save_scan_result_error` `pub(crate)` (para tests) o mantener `fn` y los tests acceden desde el mismo módulo `mod tests` |
| `src-tauri/src/commands.rs:24-29` | Modify | Espejar docstring de `Arc<Mutex<...>>` contrato (desde `ports.rs:627-635`) |

**Total PR-A**: ~150-200 líneas (dentro de budget de 400).

### PR-B (refactor)

| Archivo | Acción | Descripción |
|---|---|---|
| `src-tauri/src/ipc_error.rs` | Create | Helper `to_ipc_error` (~15 líneas) |
| `engine/src/ai/service.rs:1-52` | Modify | Agregar `AIServicePort` trait + impl para `AIService<R>`; renombrar métodos a `*_with_context` con signatura ampliada |
| `engine/src/ai/service.rs:54-181` | Modify | Agregar tests para los 2 métodos del port |
| `engine/src/ports.rs:68-78, 162-172, 335-345, 533-545, 636-677` | Modify | 4 adapters: agregar `from_arc(Arc<DbPool>)` constructor; cambiar struct a `'static` con `Arc<DbPool>` interno |
| `src-tauri/src/commands.rs:30-38` | Modify | Reemplazar 5 campos de `AppState` con 6 `Arc<dyn ...>` (ver Decisión 7) |
| `src-tauri/src/commands.rs:61-668` | Modify | 22 comandos: 1 línea cada uno (cambio a `state.foo_repo.clone()`) + reemplazar 37 `.map_err(\|e\| e.to_string())` con `to_ipc_error` |
| `src-tauri/src/commands.rs:240-317` | Modify | `explain_node`: el cuerpo se reduce a 3-5 líneas que delegan a `state.ai_service.explain_node_with_context(...)` |
| `src-tauri/src/commands.rs:319-398` | Modify | `chat`: análogo |
| `src-tauri/src/lib.rs:48-54` | Modify | Composition root: construir 6 `Arc<dyn ...>` desde los adapters |
| `src-tauri/src/commands.rs:510` | Modify | Eliminar `use engine::ports::WorkspaceRepositoryAdapter;` (ya viene vía `Arc<dyn ...>`) |
| `src/services/{ai,project,graph,snapshot,analysis}Service.ts` | Delete | 5 archivos, 257 líneas |
| `src/services/__tests__/services-boundary.test.ts` | Rename → Move | → `src/lib/__tests__/tauri-api-bridge.test.ts`, reducido |
| `src/hooks/{useAI,useAIConfig,useGraph,useNodeDetails,useNodeOutline,useProject,useArchitecture,useExport}.ts` | Modify | 9 imports migrados a `@/lib/tauri-api` |
| `src/stores/useSnapshotStore.ts:7` | Modify | Import migrado a `@/lib/tauri-api` |
| `src/lib/__tests__/tauri-api-bridge.test.ts` | Create (post rename) | Static-guard test: `rg "from.*services" src/` debe dar 0 |
| `engine/src/services/*.rs` | Modify | Las 4 services aceptan `Arc<dyn ...>` en lugar de `Adapter` o `&DbPool` |

**Total PR-B**: ~400-500 líneas (justifica chained PR).

## Interfaces / contratos

### `IpcErrorPayload` (sin cambios, ya en `engine/src/lib.rs:74-80`)

```rust
#[derive(Debug, serde::Serialize)]
pub struct IpcErrorPayload {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
```

### `to_ipc_error` (nuevo, en `src-tauri/src/ipc_error.rs`)

```rust
use engine::AppError;

/// Único punto de conversion AppError -> wire format.
/// Serializa el IpcErrorPayload (via impl Serialize for AppError) a JSON string.
pub(crate) fn to_ipc_error(e: AppError) -> String {
    serde_json::to_string(&e).unwrap_or_else(|_| e.to_string())
}
```

### `AIServicePort` (nuevo, en `engine/src/ai/service.rs`)

```rust
pub(crate) trait AIServicePort: Send + Sync {
    async fn explain_node_with_context(
        &self,
        config: &AIConfig,
        file_info: &FileInfo,
        file_content: &str,
        graph: &GraphData,
        outline: &[OutlineItem],
    ) -> crate::Result<NodeExplanation>;

    async fn chat_with_context(
        &self,
        config: &AIConfig,
        project_id: &str,
        root_path: &str,
        file_contents: &[(String, String)],
        graph: &GraphData,
        history: &[ChatMessage],
        new_user_message: &str,
    ) -> crate::Result<ChatResponse>;
}
```

### `Adapter::from_arc` (nuevo, en `engine/src/ports.rs`, 4 instancias)

```rust
impl ScanRepositoryAdapter {
    pub fn new(pool: &DbPool) -> Self { /* existente, para tests */ }
    pub fn from_arc(pool: Arc<DbPool>) -> Self { /* nuevo, para AppState */ }
}
// Idem para GraphRepositoryAdapter, WorkspaceRepositoryAdapter, AnalysisRepositoryAdapter.
```

## Estrategia de testing

| Capa | Qué testear | Cómo |
|---|---|---|
| **Unit (Rust)** | `is_root_path_conflict` y `map_save_scan_result_error` (7 tests movidos) | `#[test]` en `engine/src/services/scan_service.rs::tests`; cobertura: positivo, negativo, case-sensitivity, otros constraints, empty root_path |
| **Unit (Rust)** | `AIServicePort::explain_node_with_context` y `chat_with_context` | Mock `AIProviderResolver` con `TestProvider` (reusar patrón `service.rs:62-105`); verificar que `ContextBuilder` se invoca con los args correctos |
| **Unit (Rust)** | `Adapter::from_arc` y `Adapter::new` | `cargo test` sobre `engine/tests/ports_test.rs` extendido con un test específico de `from_arc` que verifique que el `Arc<DbPool>` se comparte entre dos adapters |
| **Contract (Rust)** | Serialización de `AppError` a `IpcErrorPayload` JSON | `engine/tests/error_contract_test.rs` (existente, sin cambios); agregar aserción de que `serde_json::to_string(&AppError::FileNotFound("x".into()))` produce el string esperado |
| **Integration (TS)** | `toApiError` parsea `IpcErrorPayload` | `src/lib/__tests__/tauri-api-bridge.test.ts` (nuevo, reducido): 1 test del path estructurado, 1 smoke test de la API, 1 static-guard test |
| **Static guard** | `npm run check:arch` detecta regresiones | `scripts/ci/check-architecture.mjs --self-test` con fixtures; corre en CI |
| **Static guard** | Ningún import de `src/services/` queda | `rg "from ['\"]@?\.?\.?/services" src/` agregado como step en CI; corre en `lint-and-typecheck` job |

## Migración / rollout

**Estrategia**: 2 PRs encadenados (PR-A → PR-B), mergeados secuencialmente a `main`. Cada PR deja la base en verde y el review budget respetado (PR-A ≤200, PR-B ≤500).

**PR-A** (`feat/pre-wave-2-pr-a-foundations`, ya creada desde `main@04e4c73`):
- PR-Branch: `feat/pre-wave-2-pr-a-foundations` (existe).
- Rebase: innecesario (rama limpia).
- Review: 1 reviewer basta; el cambio es no-breaking para consumers (los traits siguen siendo accesibles desde dentro del crate `engine`; los `Adapter` siguen siendo `pub`).

**PR-B** (`feat/pre-wave-2-pr-b-refactor`, se crea después de merge de PR-A):
- Base: `main` post-merge de PR-A.
- Strategy: 6 commits lógicos internamente (ver Decisión 7), pero un solo PR para revisión.
- Review: 2 reviewers recomendado (refactor grande, toca 22 comandos + composition root).
- Atomicidad: el spec `error-contract/spec.md:38-50` exige atomicidad backend+frontend+tests. El commit que actualiza `commands.rs` (37 reemplazos `.map_err`) y `tauri-api.ts` (parser ya está OK) y los test fixtures debe ser **un solo commit** dentro de PR-B.

**Feature flags**: no se requieren. El cambio no tiene runtime flags porque el contrato es binario (backend emite JSON o no).

**Rollback**:
- PR-A: revert del merge commit. `pub(crate)` no rompe callers internos verificados; el CI guard se puede desactivar via `npm pkg delete scripts.check:arch` sin re-deploy. Riesgo bajo.
- PR-B: revert del merge commit. Toca los mismos archivos; no hay migraciones de esquema. Riesgo medio (rollback mecánico, extenso en líneas). Plan B: cerrar PR-B como `draft`, dejar PR-A mergeado, reabrir wave 2 con scope revisado.

## Preguntas abiertas

- [ ] **¿El test de self-test del CI guard corre en CI o solo localmente?** Recomendación: ambos. El step en CI invoca `npm run check:arch -- --self-test` que corre las fixtures y falla si el script está roto. Sin esto, un script que silenciosamente deja de detectar patrones pasa CI.
- [ ] **¿Se renombra `map_save_scan_result_error` al moverlo a `engine::services`?** Recomendación: NO, mantener el nombre para minimizar la diff de los tests movidos; el comportamiento ya está documentado en el docstring (`scan_service.rs:313-316`).
- [ ] **¿`AIServicePort` debe ser `pub` o `pub(crate)`?** El spec dice `pub(crate)`. Si en el futuro se quiere exponer el trait a crates externos (e.g., `codeatlas-application`), se cambia a `pub` con un PR dedicado. Para este change: `pub(crate)`.

**Estado**: el diseño está listo para aprobación del usuario. Las decisiones de mayor consecuencia son:
1. **`to_ipc_error` en archivo nuevo `src-tauri/src/ipc_error.rs`** (vs inline en `commands.rs`): preferido por limpieza, pero mueve una decisión de organización.
2. **Secuencia 6 pasos en PR-B** (vs big-bang atómico): preferido por revisabilidad, pero requiere disciplina de commits.
3. **Política estricta del CI guard (no `arch-allow`)**: preferido por simplicidad, pero requiere PR con label para excepciones, lo cual es fricción intencional.
