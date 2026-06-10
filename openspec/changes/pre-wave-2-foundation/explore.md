# Exploración: pre-wave-2-foundation

**Rama**: `feat/pre-wave-2-foundation` (limpia, basada en `main` post-merge de wave 1)
**Fecha**: 2026-06-08
**Autor**: sdd-explore (sub-agente)
**Modo**: A1 (interactivo)

---

## 1. Planteamiento del problema

La primera ola de migración hexagonal (`hexagonal-architecture-migration`, merge PR #3) dejó la capa de presentación con fugas estructurales que la bloquean para la ola 2. Aunque los puertos canónicos (`ScanRepository`, `GraphRepository`, `WorkspaceRepository`, `AnalysisRepository`, `AppStatePort`) y los servicios (`ScanService`, `GraphService`, `WorkspaceService`, `AnalysisService`) están bien definidos en `engine/src/ports.rs:37, 139, 224, 499, 607` y `engine/src/services/`, la capa Tauri sigue orquestando lógica de negocio inline en `src-tauri/src/commands.rs`. Concretamente:

- `explain_node` (commands.rs:240-317) y `chat` (commands.rs:319-398) leen archivos del disco con `std::fs::read_to_string` (commands.rs:269, 356), instancian `ProjectRepository::new(&state.db)` (commands.rs:258, 338) y construyen `GraphData` por defecto — 160 líneas de orquestación que deberían vivir en un caso de uso de aplicación, no en la frontera IPC.
- `AppState` (commands.rs:30-38) sigue exponiendo `db: DbPool` y `ai_service: engine::ai::AIService` concretos, en vez de `Arc<dyn ...>` sobre los puertos. Esto acopla la presentación al esquema SQLite y a la estructura concreta del servicio de IA, exactamente lo contrario de lo que la arquitectura hexagonal busca.
- Los traits de los puertos están como `pub` (ports.rs:37, 139, 224, 499, 607) cuando solo se consumen dentro del crate `engine`; deberían ser `pub(crate)` para reforzar la frontera.
- El contrato de errores estructurados prometido en `openspec/specs/error-contract/spec.md` no se cumple: las 37 llamadas a `.map_err(|e| e.to_string())` en commands.rs descartan la serialización JSON de `IpcErrorPayload` que `AppError` ya implementa (lib.rs:82-150) y mandan solo el `Display` string. El parser frontend `toApiError` (tauri-api.ts:58-126) tiene la rama estructurada como código muerto.
- El frontend tiene una capa `src/services/*.ts` (5 archivos, ~257 líneas) que son re-exports 1:1 de `src/lib/tauri-api.ts`. Esta capa añade cero valor.
- Hay un bug de contrato silencioso en `NodeExplanation.nodeId`: el struct Rust tiene `pub node_id: String` con `#[serde(rename_all = "camelCase")]` (engine/src/models/ai.rs:5-14), por lo que se serializa como `nodeId`, pero la interfaz TypeScript declara `node_id: string` (types.ts:126). Esto rompe la deserialización en producción aunque los tests no lo detecten porque los fixtures usan el campo snake_case incorrecto.

**Por qué bloquea la ola 2**: la ola 2 va a partir el crate `engine` en `codeatlas-domain`, `codeatlas-application` e `codeatlas-infrastructure`. Si `AppState` sigue exponiendo `DbPool` y `AIService` concretos, los adapters van a tener que arrastrar referencias al crate de infraestructura. Si los traits son `pub` en vez de `pub(crate)`, el corte de visibilidad va a requerir renombrados masivos. Si las 160 líneas de lógica de IA viven en `commands.rs`, van a tener que migrar a `application` después, en vez de estar ya en el lugar correcto.

**Cómo se ve "bien"**: `AppState` con `Arc<dyn ScanRepository>`, `Arc<dyn AIService>`, etc.; comandos Tauri de 5-10 líneas que solo extraen estado, delegan y mapean errores; puertos con `pub(crate)`; un helper `to_ipc_error(e: AppError) -> String` en `commands.rs` que reemplace todos los `.map_err(|e| e.to_string())`; tests que verifiquen el contrato JSON a través de la frontera.

---

## 2. Archivos y sitios afectados

Inventario basado en `grep -n` y `rg`. Cada ítem del scope tiene un footprint concreto:

### Item 1: Bug `NodeExplanation.nodeId`
- `src/lib/types.ts:125-131` — declaración de la interfaz con `node_id` (incorrecto)
- `src/services/__tests__/services-boundary.test.ts:168, 305, 404` — fixtures con `node_id` (incorrecto)
- `src/hooks/__tests__/useAI-corrective.test.ts:242, 258` — fixtures con `node_id` (incorrecto)
- `engine/src/services/analysis_service.rs:116` — línea que ya emite `"nodeId"` manualmente (evidencia de que camelCase es el contrato real)
- `engine/src/models/ai.rs:5-14` — definición Rust con `#[serde(rename_all = "camelCase")]` (es la fuente de verdad)

### Item 2: Mover `explain_node` a `AIService::explain_node_with_context`
- `src-tauri/src/commands.rs:240-317` — 78 líneas de lógica de orquestación a extraer
- `engine/src/ai/service.rs:1-52` — `AIService` actual; agregar `explain_node_with_context`
- `src-tauri/src/commands.rs:258` — `let repo = ProjectRepository::new(&state.db)` (leak concreto)
- `src-tauri/src/commands.rs:269` — `std::fs::read_to_string(&file_path)` (filesystem en presentación)
- `src-tauri/src/commands.rs:291-300` — `ContextBuilder::build_node_context[_with_outline]` (uso OK; lo que se mueve es la decisión de cuál llamar)
- `src-tauri/src/commands.rs:303-310` — filtro de edges (lógica de dominio)

### Item 3: Mover `chat` a `AIService::chat_with_context`
- `src-tauri/src/commands.rs:319-398` — 80 líneas análogas a `explain_node`
- `engine/src/ai/service.rs:1-52` — agregar `chat_with_context` (o nuevo método)
- `src-tauri/src/commands.rs:338, 341-348, 351-360, 363-372, 376-382, 385-391` — sub-bloques a mover
- `src-tauri/src/commands.rs:387-391` — construcción manual de `ChatMessage` con `uuid::Uuid::new_v4()` y `chrono::Utc::now()` (clock leak menor)

### Item 4: Reemplazar `AppState` con `Arc<dyn ...>`
- `src-tauri/src/commands.rs:30-38` — definición de `AppState` (5 campos a cambiar)
- `src-tauri/src/commands.rs:61, 80, 101, 140, 159, 180, 201` — 7 comandos de `ScanService`/`GraphService` que hacen `&state.db` → deben extraer el `Arc<dyn>`
- `src-tauri/src/commands.rs:240, 320` — comandos AI que instancian `ProjectRepository::new(&state.db)` directamente
- `src-tauri/src/commands.rs:411, 426, 441, 457` — 4 comandos de `AnalysisService`
- `src-tauri/src/commands.rs:518-523, 530, 537, 543, 554, 564, 576, 588, 602, 613, 624, 640, 650, 661` — macro `workspace_service!` y 13 comandos de `WorkspaceService`
- `src-tauri/src/lib.rs:48-54` — punto de composición donde se construye `AppState` (wiring de Arc's)
- `engine/src/ports.rs:68-78, 162-172, 335-345, 533-545, 636-677` — constructores `Adapter::new(pool)` que reciben `&DbPool`; deben poder construirse también desde `Arc<dyn ...>` (¿necesitamos un constructor `from_arc` o un trait `From<Arc<dyn ...>>`?)

### Item 5: `pub(crate)` en traits
- `engine/src/ports.rs:37` — `pub trait ScanRepository`
- `engine/src/ports.rs:139` — `pub trait GraphRepository`
- `engine/src/ports.rs:224` — `pub trait WorkspaceRepository`
- `engine/src/ports.rs:499` — `pub trait AnalysisRepository`
- `engine/src/ports.rs:607` — `pub trait AppStatePort`
- ⚠️ Bloqueante técnico: si bajamos la visibilidad, los servicios (`engine/src/services/*.rs`) deben seguir compilando. Hay que verificar que ningún test externo ni `src-tauri/commands.rs` use estos traits directamente (verificar con `rg "use engine::ports::" src-tauri/`). El `grep` actual muestra un solo hit en `src-tauri/src/commands.rs:510` para `WorkspaceRepositoryAdapter` (el struct, no el trait), así que el cambio debería ser seguro.

### Item 6: CI grep guard
- Script nuevo en `scripts/ci/check-architecture.sh` (o `.js` invocado desde `package.json`)
- Verifica: `rg "use engine::db::" src-tauri/src/commands.rs` → debe ser 0
- Verifica: `rg "engine::ai::(anthropic|resolved|provider::AIProvider)" src-tauri/src/commands.rs` → debe ser 0
- Verifica: `rg "use engine::ai::AIService" src-tauri/src/commands.rs` → debe ser 0 (después de Item 4, AIService se inyecta por el port)
- Verifica: `rg "\.map_err\(\|e\| e\.to_string\(\)\)" src-tauri/src/commands.rs` → debe ser 0 (después de Item 7)
- Integración: añadir step en `.github/workflows/ci.yml` que corra el script antes de `lint-and-typecheck` (o como parte de `rust-backend`)

### Item 7: Contrato de errores
- `src-tauri/src/commands.rs:71, 93, 109, 152, 170, 194, 217, 227, 236, 247, 251, 263, 277, 316, 327, 331, 343, 348, 365, 397, 420, 436, 451, 467, 532, 539, 550, 560, 572, 582, 598, 609, 620, 632, 646, 657, 668` — 37 ocurrencias de `.map_err(|e| e.to_string())`
- `engine/src/lib.rs:74-80` — `IpcErrorPayload` ya está definido; hay que crear un helper `to_ipc_error(e: AppError) -> String` que serialice el `IpcErrorPayload` a JSON string (que es como Tauri lo transporta)
- `src/lib/tauri-api.ts:58-126` — el parser ya parsea JSON estructurado; debe seguir funcionando
- `src/lib/__tests__/tauri-api.test.ts:11-280` — los tests verifican el contrato JSON; deberían seguir pasando
- ⚠️ `commands.rs:256, 336` — `"AI not configured".to_string()` y `format!("File not found: {}", node_id)` (líneas 264, 344) son strings sin código; el helper tiene que mapearlos a `AppError::AIUnavailable`/`AppError::FileNotFound` antes de serializar

### Item 8: Eliminar `src/services/*.ts`
- `src/services/aiService.ts:1-50` — 50 líneas de re-exports
- `src/services/projectService.ts:1-68` — 68 líneas
- `src/services/graphService.ts:1-72` — 72 líneas
- `src/services/snapshotService.ts:1-38` — 38 líneas
- `src/services/analysisService.ts:1-29` — 29 líneas
- Call sites a migrar (verificado con `rg "from.*services/" src/`):
  - `src/hooks/useAI.ts:7` — `import { explainNode, chat } from '../services/aiService'`
  - `src/hooks/useAIConfig.ts:5` — `import { configureAI as _configureAI, getAIConfig as _getAIConfig } from '../services/aiService'`
  - `src/hooks/useGraph.ts:5` — `import { getGraph, searchNodes } from '../services/graphService'`
  - `src/hooks/useNodeDetails.ts:5` — `import { getNodeDetails as _getNodeDetails } from '../services/graphService'`
  - `src/hooks/useNodeOutline.ts:5` — `import { getNodeOutline as _getNodeOutline } from '../services/graphService'`
  - `src/hooks/useProject.ts:11-12` — `import {...} from '../services/projectService'` + `import { getGraph as _getGraph } from '../services/graphService'`
  - `src/hooks/useArchitecture.ts:9` — `import {...} from '../services/projectService'`
  - `src/hooks/useExport.ts:5` — `import { exportView } from '../services/analysisService'`
  - `src/stores/useSnapshotStore.ts:7` — `import { createSnapshot, listSnapshots, getSnapshot } from '../services/snapshotService'`
- `src/services/__tests__/services-boundary.test.ts:1-430` — el test mockea `tauri-api` pero los `describe` están en términos de "Service contracts" y "Hook contracts"; hay que decidir si se renombra a `tauri-api-bridge.test.ts` o se borra (recomendación: renombrar y reducir a tests del parser de errores + un smoke test de las funciones `tauri-api`)

### Item 9: Eliminar duplicados `is_root_path_conflict` / `map_save_scan_result_error`
- `src-tauri/src/commands.rs:477-501` — funciones `pub` (con `#[allow(dead_code)]`, evidencia de que ya no se usan aquí)
- `engine/src/services/scan_service.rs:320-336` — versión `fn` (no pub) que retorna `AppError` (typed)
- `src-tauri/src/commands/tests/observability_tests.rs:1-83` — 7 tests que importan `use crate::commands::{is_root_path_conflict, map_save_scan_result_error}`; si eliminamos las funciones, estos tests se rompen. Hay que migrarlos para testear la versión del servicio.
- `engine/src/services/scan_service.rs:171-181, 241-251` — call sites dentro del servicio (los que importan)
- Decisión arquitectónica: el servicio ya tiene la lógica correcta. El duplicado en `commands.rs` quedó como residuo de antes de la migración. La solución es: (a) eliminar las funciones de `commands.rs` y el test, (b) los call sites en `commands.rs` que invocaban `map_save_scan_result_error` ya no existen (verificado con `rg`, el último call site en `commands.rs` para estas funciones no se encuentra; estaban como `#[allow(dead_code)]` desde antes).

### Item 10: Documentar contrato `Arc<Mutex<...>>`
- `src-tauri/src/commands.rs:24-29` — bloque de comentarios actual sobre `Arc<Mutex<T>>`
- `engine/src/ports.rs:632-635` — comentario análogo en `AppStatePortAdapter` (el que se debe espejar)
- Es un cambio puramente documental; la línea de comentario a añadir es ~10 líneas


---

## 3. Patrones existentes que podemos reusar

### ¿Cómo se construye `AIService` hoy?
`AIService<R = ProviderFactory>` (engine/src/ai/service.rs:8) es genérico sobre el resolver, no es un trait. El default usa `ProviderFactory` (service.rs:12-18). La inyección actual es directa: `engine::ai::AIService::default()` (lib.rs:53). Para Item 4 necesitamos decidir: (a) convertirlo en un trait, o (b) pasar `Arc<AIService>` directamente.

**No hay un port trait para `AIService`**. Esto significa que Item 4 requiere trabajo nuevo. Opciones:

- **Opción A** (mínima, ~30 líneas): agregar `pub trait AIServicePort: Send + Sync` en `engine/src/ports.rs` con un método por cada caso de uso (`async fn explain_node(...)`, `async fn chat(...)`). El struct `AIService` implementa el trait trivialmente. `AppState` guarda `Arc<dyn AIServicePort>`.
- **Opción B** (intermedia, ~60 líneas): usar `Arc<AIService>` directamente; no requiere trait, pero pierde hexagonal purity (la presentación conoce la estructura concreta).
- **Opción C** (pura, ~150 líneas): introducir `ExplainNodeUseCase` y `ChatUseCase` que toman un `Arc<dyn AIProviderResolver>` o `Arc<dyn AIServicePort>` y encapsulan toda la orquestación (lectura de archivos, construcción de contexto, etc.). Esta es la que mejor se alinea con la arquitectura target, y de hecho es lo que pide el scope en los ítems 2 y 3.

Recomendación: **Opción C**, porque ya hay 160 líneas de orquestación que mover. Crear dos use cases vacíos solo para no ganar hexagonal purity sería peor que la situación actual.

### ¿Cómo funciona la serialización de `IpcErrorPayload`?
`engine/src/lib.rs:74-80` define `IpcErrorPayload { code, message, details }`. La impl `Serialize` para `AppError` (lib.rs:82-150) construye el payload por variante. **El problema**: cuando un comando retorna `Result<T, String>`, el `String` que cruza la frontera IPC es el `Display` del `AppError`, no la serialización JSON del `IpcErrorPayload`. Para que el contrato funcione, hay que serializar a JSON string manualmente antes de retornar el error.

El parser `toApiError` (tauri-api.ts:58-126) ya parsea JSON estructurado, así que la rama de fallback queda para compatibilidad con errores pre-migración.

### ¿Hay ejemplos de `pub(crate)` en `engine`?
**No.** `rg "pub\(crate\)" engine/src/` no devuelve resultados. El primer uso establecerá el precedente. No hay riesgo, solo hay que validar que el compilador no se queje.

### ¿Hay constructores `::new(pool)` que podamos reemplazar con constructores trait-object?
Sí, los 5 adapters:
- `ScanRepositoryAdapter::new(&'pool DbPool)` — ports.rs:73
- `GraphRepositoryAdapter::new(&'pool DbPool)` — ports.rs:167
- `WorkspaceRepositoryAdapter::new(&'pool DbPool)` — ports.rs:340
- `AnalysisRepositoryAdapter::new(&'pool crate::db::DbPool)` — ports.rs:539
- `AppStatePortAdapter::new(Mutex<...>, Mutex<...>, Mutex<...>)` — ports.rs:643 y `from_arc_refs(...)` — ports.rs:666

El patrón actual es: el adapter envuelve un `ProjectRepository<'pool>`, y el `'pool` lifetime es lo que permite evitar `Arc`. Para que `AppState` pueda guardar `Arc<dyn ...>`, necesitamos que los adapters sean `Send + Sync` sin un lifetime. Esto se logra envolviendo el `DbPool` en un `Arc` y eliminando el lifetime, o usando un trait object interno. Estimación: ~80-120 líneas de refactor en `ports.rs` (cambiar la signatura de los 5 adapters para que sean `'static` o contengan `Arc<DbPool>` internamente).

**Alternativa más simple**: introducir un constructor `Adapter::from_arc(Arc<DbPool>)` para cada adapter, y mantener `Adapter::new(&DbPool)` para los tests internos. Esto preserva la compatibilidad con los tests existentes (`engine/src/services/scan_service.rs:380, 410, 433, 468, 487`) y reduce el blast radius.

---

## 4. Riesgos y desconocidos

### Riesgo 1 (alto): Cascada de firmas al cambiar `AppState` a `Arc<dyn ...>`
Cada uno de los 22 comandos en `commands.rs` extrae `state: State<'_, AppState>`. La nueva forma requiere `Arc<dyn ScanRepository>` etc. — los servicios deben aceptar `Arc<dyn ...>`, no `&DbPool`. Estimación: **~200-300 líneas tocadas** solo en signaturas (services, commands, lib.rs, ports). Si los services no aceptan `Arc<dyn ...>`, los commands van a tener que hacer `.clone()` en cada llamada, lo cual es feo. Necesitamos un cambio quirúrgico en los constructores de los services para que el campo sea `Arc<dyn ScanRepository>` y los adapters se inyecten una sola vez en composición (lib.rs).

### Riesgo 2 (medio): `IpcErrorPayload` puede tener variantes que el frontend no conoce
El catálogo actual de codes (lib.rs:88-146) es: `PROJECT_NOT_FOUND, FILE_NOT_FOUND, SCAN_TIMEOUT, DATABASE, AI_UNAVAILABLE, AI_RATE_LIMITED, AI_TOKEN_LIMIT, INVALID_API_KEY, ACCESS_DENIED, INTERNAL`. El mapping en `tauri-api.ts:35-46` cubre los 10. Pero el frontend `ErrorCode` union (types.ts:158-167) es: `PATH_NOT_FOUND, PROJECT_EXISTS, ACCESS_DENIED, SCAN_TIMEOUT, INVALID_KEY, UNREACHABLE, RATE_LIMITED, TOKEN_LIMIT, INTERNAL`. **`PROJECT_EXISTS` no tiene contraparte en el backend** — solo se construye en el fallback legacy de `toApiError` (tauri-api.ts:99). Si el scope introduce `AppError::ProjectExists`, hay que mapearlo; si no, se queda solo en la rama legacy. Recomiendo dejarlo fuera del scope.

### Riesgo 3 (medio): Eliminar `src/services/*.ts` puede romper imports no grepeados
El `rg "from.*services/"` solo cubre imports directos desde el mismo nivel (`hooks/`, `stores/`). No cubre imports con alias (`@/services/...` configurado en `tsconfig.json`) ni re-exports en barrel files. Hay que verificar `tsconfig.json` antes de borrar. Estimación: ~5% de probabilidad de encontrar un import sorpresa.

### Riesgo 4 (bajo): Mover lógica expone que `AIService` hace demasiado
Si después de mover las 160 líneas `AIService` tiene 6 métodos públicos (4 use cases + 2 utility), es señal de que hay que partirlo. La pregunta abierta #1 ya cubre esta decisión. Estimación: si vamos por Opción C, no hay problema; si vamos por Opción A, queda un `AIService` con responsabilidad mixta.


---

## 5. Estimación de tamaño de diff por ítem del scope

| Item | Descripción | Estimación | Notas |
|------|-------------|------------|-------|
| 1 | Bug `nodeId` | ~10 líneas + 5 fixtures | Trivial. 1 archivo TS + 1 archivo test + 1 test hooks |
| 2 | Mover `explain_node_with_context` | ~120 líneas | 80 movidas + ~30 de glue + ~10 de tests |
| 3 | Mover `chat_with_context` | ~120 líneas | 80 movidas + ~30 de glue + ~10 de tests |
| 4 | `AppState` con `Arc<dyn ...>` | **~250-350 líneas** | ⚠️ El más grande. Toca services, commands, lib.rs, ports. Flagged: excede 100 |
| 5 | `pub(crate)` en 5 traits | ~5 líneas | Trivial, pero requiere verificar uso externo |
| 6 | CI grep guard | ~50 líneas | script bash + step en ci.yml |
| 7 | Helper `to_ipc_error` + reemplazar 37 `.map_err` | ~80 líneas | 1 helper + 37 reemplazos mecánicos + tests |
| 8 | Eliminar `src/services/*.ts` | ~30 líneas (netas) | 5 archivos borrados + 9 imports actualizados + test renombrado |
| 9 | Eliminar duplicados de error helpers | ~15 líneas netas | Borrar funciones + mover tests |
| 10 | Comentario `Arc<Mutex<...>>` | ~10 líneas | Solo docs |

**Total estimado**: **~700-1000 líneas** (con eliminación de duplicados). Ajustado al review budget de 400 líneas, este cambio excede por ~2x. **Recomendación**: partir en 2 PRs encadenados:
- **PR-A** (~400 líneas): Items 1, 5, 6, 9, 10 + parcial de 7 (helper creado, 20 reemplazos hechos). Sin cambios de signatura.
- **PR-B** (~500 líneas): Items 2, 3, 4, 7 (resto) + 8.

Esto encaja con la estrategia `chained_pr_strategy: auto-forecast` declarada en `openspec/config.yaml:21`.

**Items que exceden 100 líneas individualmente**: Item 4 (250-350). Items 2 y 3 (120 cada uno, en el límite).

---

## 6. Impacto en tests

### Tests que necesitan cambio directo
- `src/lib/__tests__/tauri-api.test.ts:1-387` — **sin cambios** esperados. El helper `to_ipc_error` no rompe el contrato existente (de hecho, lo honra). Riesgo: si los tests actualmente pasan por la rama legacy (tauri-api.ts:96-125), pueden empezar a fallar porque la rama JSON ahora se ejecuta. Hay que verificar con `npm test` después del cambio.
- `src/services/__tests__/services-boundary.test.ts:1-430` — **renombrar** a `tauri-api-bridge.test.ts` y reducir. Los tests de "Service contracts" pasan a ser tests directos de `tauri-api` (mismo mock, mismo comportamiento esperado). Los fixtures de `node_id` se actualizan a `nodeId`. Estimación: ~300 líneas reducidas a ~150.
- `engine/tests/error_contract_test.rs:1-218` — **sin cambios**. La impl `Serialize` para `AppError` no se toca.
- `src/hooks/__tests__/useAI-corrective.test.ts:230-280` — actualizar fixtures de `node_id` a `nodeId`. 2 líneas.

### Tests que podrían romperse
- `src-tauri/src/commands/tests/observability_tests.rs:1-83` — depende de `crate::commands::{is_root_path_conflict, map_save_scan_result_error}`. Si se eliminan, este test se rompe. **Migrarlo** a `engine/src/services/scan_service.rs` y testear contra la versión interna (que ahora hay que exponer `pub(crate)` para los tests).
- `engine/src/services/scan_service.rs:338-495` — los tests internos de `ScanService` (4 tests) usan `crate::ports::ScanRepositoryAdapter::new(&pool)` y `AppStatePortAdapter::new(...)`. Si cambiamos las signaturas de los adapters en Item 4, estos tests pueden romperse. Ver sección 3: la opción "Adapter::from_arc(Arc<DbPool>)" preserva compatibilidad.

### Tests que NO necesitan cambio
- `engine/src/ai/service.rs:54-181` — los tests de `AIService` ya usan `TestResolver` con `TestProvider`. Mover lógica a `explain_node_with_context` puede requerir tests nuevos (no reemplazar los existentes).
- `engine/tests/ai_boundary_test.rs:1-?` — verifica que el boundary del módulo AI se respeta; no le afecta.
- `engine/tests/ports_test.rs`, `engine/tests/scan_service_test.rs`, etc. — sin cambios esperados a nivel de API pública.

### Tests a crear
- Para `AIService::explain_node_with_context` y `chat_with_context`: tests con un `MockScanRepository` (reutilizar el patrón de `engine/src/services/scan_service.rs:346-372`).
- Para el CI grep guard: un test que verifique que el script falla cuando encuentra las condiciones prohibidas (puede ser un test bash simple).

---

## 7. Preguntas abiertas para el usuario (negocio, no mecánicas)

1. **Port trait para `AIService`**: Hoy `AIService` es un struct genérico, no un trait. Item 4 necesita que viva detrás de un trait para `Arc<dyn ...>`. Tres caminos:
   - (a) Crear un `AIServicePort` trait delgado con 2 métodos (`explain_node_with_context`, `chat_with_context`). Mínimo, ~30 líneas.
   - (b) Usar `Arc<AIService>` directamente. Sin trait, pero la presentación conoce el struct concreto.
   - (c) Crear dos use cases (`ExplainNodeUseCase`, `ChatUseCase`) que toman `Arc<dyn AIServicePort>` o el resolver. Más puro, ~150 líneas, alineado con arquitectura target de ola 2.
   - **Recomendación**: (c). El costo extra (~120 líneas) se paga solo porque ya hay 160 líneas de orquestación que mover a algún lado.

2. **Tests frontend con strings de error**: `tauri-api.test.ts` y `services-boundary.test.ts` actualmente verifican la rama legacy de `toApiError` (string heuristics). Después de Item 7, los errores vendrán como JSON estructurado. ¿Actualizamos los tests en el mismo PR (aceptar el cambio como breaking pero atómico), o aceptamos que fallen y los arreglamos en un follow-up?
   - **Recomendación**: atómico. El spec de error-contract ya exige atomic rollout (spec.md:120-128).

3. **CI guard**: ¿Script bash en `scripts/ci/` con un step dedicado en `.github/workflows/ci.yml`, o un script JS invocado desde `package.json` (más liviano, sin tocar el workflow)?
   - **Recomendación**: package.json. La CI ya corre `npm run typecheck` y `npm run lint`; agregar `npm run check:arch` es 1 línea en `package.json` y evita modificar el workflow. El script vive en `scripts/ci/check-architecture.mjs`.

4. **Estrategia de chained PR**: El diff total estimado (~700-1000 líneas) excede el `review_budget_changed_lines: 400` declarado en config.yaml. ¿Lo partimos en 2 PRs encadenados (PR-A: foundations, PR-B: refactor), o forzamos un solo PR grande con justificación explícita?
   - **Recomendación**: 2 PRs encadenados. La estrategia `chained_pr_strategy: auto-forecast` ya está declarada.

5. **Item 1 (bug `nodeId`)**: Es un fix independiente de 1 línea. ¿Lo extraemos como hotfix antes de empezar el change, o lo incluimos en PR-A? Como hotfix se puede mergear a main en 5 minutos; incluido en PR-A va a estar atado al resto.
   - **Recomendación**: extraer como hotfix a main antes de empezar. Aísla el riesgo, reduce el PR-A, y desbloquea la verificación de los demás cambios (los tests `useAI` y `services-boundary` van a empezar a usar el campo correcto).


---

## 8. Estado de preparación para la siguiente fase

### Listo para `propose`
- ✅ Planteamiento del problema articulado
- ✅ Footprint de archivos mapeado (10 ítems × archivos con `file:line`)
- ✅ Patrones existentes identificados
- ✅ Riesgos catalogados (4 riesgos, 1 alto)
- ✅ Estimación de diff honesta (con flag para chained PR)
- ✅ Impacto en tests analizado
- ✅ Preguntas abiertas para el usuario (5, todas de negocio)

### Bloqueos para `propose`
- ⏸️ Esperar respuesta a las 5 preguntas abiertas (especialmente #1 sobre la estrategia de port para `AIService` y #4 sobre chained PR)

### Lo que NO se hace en esta fase (correcto)
- ❌ No se propone diseño (eso es `sdd-design`)
- ❌ No se crean tasks (eso es `sdd-tasks`)
- ❌ No se toca código (esto es solo exploración)

### Evidencia clave resumida
- `commands.rs` tiene 669 líneas, de las cuales ~200 son orquestación que no debería estar ahí
- 37 ocurrencias de `.map_err(|e| e.to_string())` descartan el contrato JSON de error
- 5 archivos en `src/services/` son re-exports puros (257 líneas borrables)
- Bug de contrato `nodeId`/`node_id` silencioso, no detectado por tests
- 5 traits de ports `pub` que deberían ser `pub(crate)`
- 22 comandos Tauri que dependen de un `AppState` con tipos concretos
- 2 funciones de error (`is_root_path_conflict`, `map_save_scan_result_error`) duplicadas entre commands y servicio

