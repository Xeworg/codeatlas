# CodeAtlas — Estándares de Código Reutilizable y Arquitectura

**Versión:** v1.0 (pre-SDD)  
**Idioma:** español (ES-ar)  
**Propósito:** reglas de construcción, revisión y evolución de código para CodeAtlas.  
**Audiencia:** todo desarrollador que toque el repositorio.  
**Vigencia:** desde Sprint 0 hasta cierre de v3. Se revisa al final de cada versión mayor.

---

## Tabla de contenidos

1. [Estilo de arquitectura](#1-estilo-de-arquitectura)
2. [Estructura de módulos y límites](#2-estructura-de-módulos-y-límites)
3. [Reglas de dependencia](#3-reglas-de-dependencia)
4. [Convenciones de código — Rust](#4-convenciones-de-código--rust)
5. [Convenciones de código — TypeScript / React](#5-convenciones-de-código--typescript--react)
6. [Estrategia de manejo de errores](#6-estrategia-de-manejo-de-errores)
7. [Estrategia de testing](#7-estrategia-de-testing)
8. [Disciplina de contratos API](#8-disciplina-de-contratos-api)
9. [Componentes y servicios reutilizables](#9-componentes-y-servicios-reutilizables)
10. [Anti-patrones prohibidos](#10-anti-patrones-prohibidos)
11. [CI y gates de calidad](#11-ci-y-gates-de-calidad)
12. [Checklist de revisión de PR](#12-checklist-de-revisión-de-pr)
13. [Política de refactoring](#13-política-de-refactoring)
14. [Política de rollout de cambios](#14-política-de-rollout-de-cambios)
15. [Evolución forward-compat (v2/v3)](#15-evolución-forward-compat-v2v3)
16. [Acceptance Criteria de este estándar](#16-acceptance-criteria-de-este-estándar)
17. [Apéndices](#apéndices)

---

## 1. Estilo de arquitectura

### 1.1 Arquitectura general: Clean Architecture adaptada

CodeAtlas sigue **Clean Architecture** con adaptaciones para una app Tauri monolítica (no microservicios). La misma disciplina de dependencia aplica en ambos lados (Rust y React):

```
┌──────────────────────────────────────────────┐
│              Presentation Layer               │
│  React components, Tauri commands (handlers) │
├──────────────────────────────────────────────┤
│              Application Layer                │
│  Use cases: scan_project, build_graph,       │
│  explain_node, chat                          │
├──────────────────────────────────────────────┤
│                Domain Layer                   │
│  Entities: Project, FileInfo, GraphNode,     │
│  SymbolInfo, ChatMessage                     │
│  Value Objects: NodeType, ScanStatus         │
├──────────────────────────────────────────────┤
│            Infrastructure Layer               │
│  Tree-sitter, SQLite, Tauri shell,           │
│  HTTP client (IA), keyring, filesystem       │
└──────────────────────────────────────────────┘
```

**Regla de dependencia universal:**
> Una capa externa puede depender de una interna. **Nunca al revés.**

| Capa | ¿Depende de…? |
|---|---|
| **Infrastructure** (`engine/src/db/`, `scanner/`, `ai/`) | Solo de Domain (modelos). |
| **Domain** (`engine/src/models/`) | De nada externo. Solo Rust std + `serde`. |
| **Application** (`engine/src/lib.rs`, `graph/`, `src/lib/`) | De Domain + Infrastructure (vía traits). |
| **Presentation** (`src/components/`, `src-tauri/src/main.rs`) | De Application + Domain. Nunca de Infrastructure directamente. |

### 1.2 Por qué Clean Architecture

- **Rust ya impone boundaries fuertes** con el sistema de módulos; Clean Architecture las vuelve explícitas y testables.
- **React ya separa lógica de UI** con hooks y stores; la misma disciplina se aplica al backend.
- **Forward-compat para v2/v3:** agregar detectores de arquitectura, snapshots de colaboración o dashboards ejecutivos no rompe el dominio actual.
- **Testabilidad:** la lógica de negocio (construcción de grafo, preparación de contexto IA) es testeable sin SQLite ni Tree-sitter ni HTTP.

### 1.3 Convención de capas en Rust

```rust
// ✅ CORRECTO: el dominio no conoce nada externo
// engine/src/models/project.rs
use serde::{Deserialize, Serialize};  // solo serde, NUNCA rusqlite ni reqwest

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub project_id: String,
    pub files: Vec<FileInfo>,
    pub scan_duration_ms: u64,
}

// ❌ INCORRECTO: dominio conoce base de datos
// use rusqlite::Connection;  ← PROHIBIDO en models/
```

```rust
// ✅ CORRECTO: application depende de domain + traits de infrastructure
// engine/src/lib.rs
use engine::models::ScanResult;
use engine::db::ProjectRepository;  // ← vía trait, no impl concreta

pub fn scan_project(path: &str, scanner: &dyn ProjectScanner, repo: &dyn ProjectRepository) -> Result<ScanResult> {
    // lógica de aplicación
}
```

### 1.4 Convención de capas en TypeScript/React

```
src/
├── components/   ← Presentation: solo JSX + hooks. NUNCA invoke directo a Tauri.
├── hooks/        ← Application: usa stores, llama a tauri-api.ts.
├── stores/       ← Application: estado global (Zustand).
├── lib/          ← Domain + Application: tipos, llamadas tipadas a Tauri, helpers puros.
```

**Regla:** los componentes (`components/`) no llaman `invoke()` directamente. Siempre pasan por un hook o store.

```typescript
// ❌ INCORRECTO: componente llama invoke directamente
function GraphView() {
  const data = await invoke("get_graph", { projectId }); // NO
}

// ✅ CORRECTO: componente usa hook que abstrae la llamada
function GraphView() {
  const { graphData, loading, error } = useGraph(projectId);
}
```

---

## 2. Estructura de módulos y límites

### 2.1 Módulos Rust (engine crate)

```
engine/src/
├── lib.rs                   ← API pública: re-exporta solo lo necesario
├── models/                  ← Domain (entidades, value objects, DTOs)
│   ├── mod.rs
│   ├── project.rs
│   ├── file.rs
│   ├── graph.rs
│   └── ai.rs
├── scanner/                 ← Infrastructure
│   ├── mod.rs
│   ├── walker.rs
│   └── parser.rs
├── graph/                   ← Application (lógica de negocio pura)
│   ├── mod.rs
│   ├── builder.rs
│   └── resolver.rs
├── ai/                      ← Application + Infrastructure
│   ├── mod.rs
│   ├── provider.rs          ← trait (Domain-level)
│   ├── anthropic.rs         ← impl concreta
│   └── context.rs           ← lógica de construcción de contexto
├── db/                      ← Infrastructure
│   ├── mod.rs
│   ├── schema.rs
│   └── queries.rs
└── tauri_commands.rs        ← Presentation (handlers Tauri)
```

### 2.2 Módulos TypeScript (src/)

```
src/
├── components/              ← Presentation
│   ├── layout/              ← AppShell, Sidebar, TopBar, StatusBar
│   ├── graph/               ← GraphView, GraphNode, GraphEdge, MiniMap
│   ├── panel/               ← DetailPanel, AIExplanation, SymbolList
│   ├── chat/                ← ChatPanel, ChatMessage, ChatInput
│   ├── onboarding/          ← WelcomeScreen, ApiKeySetup, ProjectSelector
│   └── common/              ← Button, Modal, Spinner, MarkdownView
├── hooks/                   ← Application
│   ├── useProject.ts
│   ├── useGraph.ts
│   ├── useAI.ts
│   └── useSettings.ts
├── stores/                  ← Application
│   ├── projectStore.ts
│   ├── graphStore.ts
│   └── chatStore.ts
├── lib/                     ← Domain + Application
│   ├── types.ts             ← tipos canónicos (source of truth)
│   ├── tauri-api.ts         ← wrappers tipados de invoke
│   └── graph-layout.ts     ← helpers puros
└── styles/
```

### 2.3 Límites duros (qué no cruza)

| Módulo | NO puede… |
|---|---|
| `engine/src/models/` | Importar de `db/`, `scanner/`, `ai/`, `tauri`. |
| `engine/src/graph/` | Importar `rusqlite`, `reqwest`, `tree_sitter`. |
| `engine/src/scanner/` | Depender de `graph/` (scanner produce datos crudos, graph los consume). |
| `src/components/` | Llamar `invoke()` directamente o importar stores. |
| `src/lib/types.ts` | Importar nada de `components/` o `hooks/`. |
| `src/stores/` | Importar componentes React. |

---

## 3. Reglas de dependencia

### 3.1 Rust: dependencias permitidas por módulo

```
models/     → (ninguna externa, solo serde + uuid)
scanner/    → models/
graph/      → models/
ai/         → models/
db/         → models/
lib.rs      → models/ + scanner/ + graph/ + ai/ + db/ (orquestación)
tauri_commands.rs → models/ + lib.rs
```

### 3.2 TypeScript: dependencias permitidas

```
types.ts        → (ninguna interna, solo types)
tauri-api.ts    → types.ts
graph-layout.ts → types.ts
stores/         → types.ts, tauri-api.ts
hooks/          → types.ts, stores/
components/     → hooks/, types.ts, components/common/
```

### 3.3 Inversión de dependencia con traits (Rust)

```rust
// engine/src/graph/builder.rs (Application)
// NO depende de scanner/ ni db/ concretos

pub trait FileProvider {
    fn list_files(&self, project_id: &str) -> Result<Vec<FileInfo>>;
    fn get_imports(&self, file_id: &str) -> Result<Vec<ImportInfo>>;
}

pub fn build_graph(provider: &dyn FileProvider, project_id: &str) -> Result<GraphData> {
    let files = provider.list_files(project_id)?;
    // construir grafo usando solo FileInfo e ImportInfo
}
```

---

## 4. Convenciones de código — Rust

### 4.1 Estilo y formato

- `cargo fmt` obligatorio en pre-commit (max_width = 100).
- `cargo clippy -- -D warnings`. PRs con warnings de clippy no mergean.

### 4.2 Nombrado

| Elemento | Convención | Ejemplo |
|---|---|---|
| Módulos / archivos | `snake_case` | `graph_builder.rs` |
| Structs / Enums / Traits | `PascalCase` | `GraphNode`, `NodeType` |
| Funciones / métodos | `snake_case` | `build_graph()`, `resolve_path()` |
| Constantes / estáticas | `SCREAMING_SNAKE_CASE` | `MAX_FILES` |

### 4.3 Manejo de Option y Result

- **Nunca usar `.unwrap()` en código de producción** salvo en tests o inicialización temprana infalible.
- Preferir `?` con `Result<T, AppError>`.
- Error types con `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Archivo no encontrado: {0}")]
    NotFound(String),
    #[error("Error de base de datos: {0}")]
    Database(String),
    #[error("Timeout de escaneo: {files_processed}/{total_files}")]
    ScanTimeout { files_processed: usize, total_files: usize },
    #[error("IA no disponible: {0}")]
    AIUnavailable(String),
    #[error("Error interno: {0}")]
    Internal(String),
}
```

### 4.4 Documentación

- Todo struct, trait, enum y función pública debe tener doc comment (`///`).
- Ejemplos de uso con `/// # Examples` cuando el uso no sea obvio.

---

## 5. Convenciones de código — TypeScript / React

### 5.1 Formato

- Prettier con `printWidth: 100`, `singleQuote: false`.
- ESLint con `@typescript-eslint` strict.

### 5.2 Nombrado

| Elemento | Convención | Ejemplo |
|---|---|---|
| Componentes React | `PascalCase` | `function GraphView() {}` |
| Hooks | `use` + `PascalCase` | `useGraph()`, `useProject()` |
| Funciones / variables | `camelCase` | `buildLayout()`, `fileCount` |
| Tipos / Interfaces | `PascalCase` | `GraphData`, `FileInfo` |

### 5.3 Componentes React

- **Un componente = un archivo.** Máximo 200 líneas.
- **Props tipadas explícitamente** (nunca `any`).
- **Custom hooks** para lógica reutilizable. Si una lógica se repite en 2+ componentes, extraer hook.
- **Estado global → Zustand.** Estado local solo cuando no necesita compartirse.

### 5.4 Zustand stores

```typescript
// Patrón: store con acciones atómicas + selectores exportados
export const useScanStatus = () => useProjectStore((s) => s.status);
export const useSelectedNodeId = () => useProjectStore((s) => s.selectedNodeId);
```

---

## 6. Estrategia de manejo de errores

### 6.1 Jerarquía

```
AppError (Rust) → serializado a JSON → ApiError (TypeScript)
```

### 6.2 Contrato de error canónico

```typescript
interface ApiError {
  code: ErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

type ErrorCode =
  | "PATH_NOT_FOUND" | "ACCESS_DENIED" | "SCAN_TIMEOUT"
  | "INVALID_KEY" | "UNREACHABLE" | "RATE_LIMITED"
  | "TOKEN_LIMIT" | "INTERNAL";
```

### 6.3 UX de errores

Cada código de error → mensaje legible + acción sugerida en la UI.

---

## 7. Estrategia de testing

### 7.1 Distribución

| Nivel | Herramienta | Meta v1 |
|---|---|---|
| Unitarios Rust | `cargo test` | ≥ 80% cobertura |
| Unitarios TS | `vitest` | ≥ 75% cobertura |
| Integración Rust | `cargo test -- --ignored` | fixtures clave |
| Integración TS | `vitest` + mock `invoke` | stores/hooks clave |
| Contratos | `cargo test -- contracts` | todos los comandos Tauri |
| E2E | Checklist manual | 10 flujos |
| Benchmarks | `cargo bench` | informativo |

### 7.2 Qué testear (no negociable)

| Si existe… | Debe tener… |
|---|---|
| Función pública en Rust | ≥ 1 test unitario |
| Hook de React | ≥ 1 test con `renderHook` |
| Endpoint Tauri | ≥ 1 test de contrato (snapshot) |
| Store de Zustand | ≥ 1 test por acción y por estado de error |
| Componente con loading/error/empty | ≥ 1 test por estado |

---

## 8. Disciplina de contratos API

### 8.1 Source of truth

Los tipos en `src/lib/types.ts` y `engine/src/models/` son **el contrato canónico**.

### 8.2 Checklist de sincronización (por PR)

- [ ] `types.ts`: interface actualizada.
- [ ] `models/*.rs`: struct actualizada con `#[derive(Serialize, Deserialize)]`.
- [ ] `#[serde(rename_all = "camelCase")]` presente.
- [ ] Campos nuevos: `Option<T>` en Rust, `field?` en TS.
- [ ] `tauri-api.ts`: firma de invoke coincide.
- [ ] Contract test snapshot actualizado.
- [ ] `docs/CHANGELOG_CONTRATOS.md` actualizado.
- [ ] Breaking changes documentados con plan de deprecación.

### 8.3 Regla de oro

> **Un cambio de tipo DEBE reflejarse en ambos lados (TS + Rust) en el MISMO PR.**

---

## 9. Componentes y servicios reutilizables

### 9.1 Definición de "reutilizable"

- Está en `src/components/common/` (UI) o es un trait público en `engine/src/`.
- No tiene estado específico de una feature.
- Recibe todo lo que necesita vía props/parámetros.
- Está documentado con JSDoc o doc comment.
- Tiene al menos 1 test unitario.

### 9.2 Componentes UI reutilizables (v1)

| Componente | Props mínimas |
|---|---|
| `<Button>` | `variant`, `size`, `disabled`, `onClick`, `children` |
| `<Modal>` | `open`, `onClose`, `title`, `children` |
| `<Spinner>` | `size` |
| `<MarkdownView>` | `content` |
| `<EmptyState>` | `icon`, `title`, `description`, `action?` |
| `<ErrorState>` | `message`, `onRetry?`, `actionLabel?` |
| `<Skeleton>` | `width`, `height`, `variant` |

### 9.3 Servicios Rust reutilizables (v1)

| Trait | Propósito |
|---|---|
| `FileProvider` | Proveer archivos e imports para construir grafo |
| `ProjectRepository` | CRUD de proyectos, archivos, símbolos |
| `AIProvider` | Abstracción de proveedor IA |
| `ContextBuilder` | Construir contexto para prompts |

### 9.4 Regla de extracción

> Si una lógica se repite en **2 o más lugares**, se extrae a componente/hook/trait/función reutilizable **antes del merge**, no después.

---

## 10. Anti-patrones prohibidos

### 10.1 Rust

| Anti-patrón | Qué hacer en su lugar |
|---|---|
| `.unwrap()` en producción | `?` con `Result<T, AppError>` |
| `String` como tipo de error | `enum AppError` con `thiserror` |
| Struct de dominio con dependencias externas | Solo `serde` en models |
| Módulo `scanner/` que importa `graph/` | Scanner produce datos crudos; graph los consume |
| `panic!()` en código de análisis | `Result::Err` con mensaje descriptivo |
| Funciones de 100+ líneas | Extraer funciones privadas de ≤ 50 líneas |

### 10.2 TypeScript / React

| Anti-patrón | Qué hacer en su lugar |
|---|---|
| `any` | Tipo explícito, `unknown` si es necesario |
| `invoke()` directo en componente | Hook o store |
| Estado duplicado en 2 stores | Una sola fuente de verdad |
| Props drilling > 3 niveles | Zustand o composición |
| Componente de 300+ líneas | Extraer subcomponentes |
| `// @ts-ignore` | Arreglar el tipo o usar `as` con comentario |
| `setTimeout` para sincronización | Estado explícito o useEffect con dependencias |

### 10.3 General

| Anti-patrón | Qué hacer en su lugar |
|---|---|
| PR de 800+ líneas | Dividir en commits atómicos o chained PRs |
| Mergear sin tests | ≥ 1 test por feature |
| "Lo arreglo después" | Issue en backlog con prioridad |
| Features de v2 coladas en PR de v1 | Mover a rama separada o backlog |
| Hardcodear valores configurables | Constante o settings |

---

## 11. CI y gates de calidad

### 11.1 Gates que bloquean merge

| Gate | ¿Bloquea? |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy -- -D warnings` | ✅ |
| `npm run lint` | ✅ |
| `cargo test` + `npm run test` | ✅ |
| Contract tests | ✅ (PRs que tocan modelos/tipos) |
| `npm run typecheck` | ✅ |
| Benchmarks | ❌ (informativo) |

---

## 12. Checklist de revisión de PR

### 12.1 Para el autor (antes de pedir review)

- [ ] Código compila (`cargo build` y `npm run build`).
- [ ] Tests pasan localmente.
- [ ] Linter limpio.
- [ ] Formato aplicado.
- [ ] Tipos sincronizados TS ↔ Rust si toca contratos.
- [ ] Nuevos endpoints/funciones públicas tienen doc comments.
- [ ] Nuevos componentes/hooks/traits tienen ≥ 1 test.
- [ ] No se introdujeron dependencias circulares.
- [ ] No hay `.unwrap()`, `any`, `@ts-ignore` sin justificación escrita.
- [ ] PR ≤ 400 líneas (o justificado y aprobado por TL).
- [ ] Si es breaking change: documentado en CHANGELOG.

### 12.2 Para el reviewer

- [ ] ¿La lógica de negocio está en la capa correcta?
- [ ] ¿Hay tests para el happy path y al menos 1 edge case?
- [ ] ¿Los errores se manejan con `AppError` / `ApiError`?
- [ ] ¿Los estados de UI (loading, empty, error) están cubiertos?
- [ ] ¿El contrato de API es backward-compatible?
- [ ] ¿Se respetan los límites de módulo?
- [ ] ¿El código es reutilizable o tiene justificación para ser específico?
- [ ] ¿Se introdujo scope creep de v2/v3? → rechazar o mover a feature branch.
- [ ] ¿Hay código duplicado que debería extraerse?

---

## 13. Política de refactoring

### 13.1 Reglas

1. **Refactoring y feature en PRs separados.**
2. **Refactoring no cambia comportamiento.**
3. **Refactoring grandes (> 400 líneas) → chained PRs o aprobación de TL.**
4. **Siempre con tests verdes antes y después.**

### 13.2 Boy scout rule

> Dejá el código un poco mejor de como lo encontraste.  
> Si ves un `unwrap()` viejo, un `any` suelto, o un componente sin test al tocarlo → arreglalo en el mismo PR si es ≤ 20 líneas. Si es más grande → issue separado.

---

## 14. Política de rollout de cambios

### 14.1 Branches

```
main          ← producción (siempre verde)
├── feat/*     ← features
├── fix/*      ← bugs
├── refactor/* ← refactors
└── docs/*     ← documentación
```

### 14.2 Flujo

```
1. Crear branch desde main.
2. Implementar con tests.
3. PR → CI verde → review → aprobación.
4. Merge a main.
```

### 14.3 Rollback

- Si merge a main rompe CI o introduce bug P0 → revertir PR completo.
- No se hace "hotfix sobre hotfix". Revertir primero, analizar después.

---

## 15. Evolución forward-compat (v2/v3)

### 15.1 Qué protegemos ahora para no romper después

| Lo que hacemos en v1 | Cómo facilita v2/v3 |
|---|---|
| `AIProvider` como trait | Agregar nuevos proveedores sin tocar dominio |
| `FileProvider` como trait | Cambiar fuente de datos sin tocar graph builder |
| `NodeType` con variantes desde v1 | Agregar tipos sin breaking change |
| `GraphData` con `generated_at` | Snapshots comparativos en v3 |
| `AppError` como enum | Agregar variantes sin romper matches existentes |
| Stores Zustand atómicos | Agregar nuevas stores sin refactorizar existentes |
| SQLite con `content_hash` | Escaneo incremental en v2 sin cambiar schema |
| Contratos versionados con deprecación | Remover campos en v2 sin breaking change repentino |

### 15.2 Qué NO hacemos en v1 (para no hipotecar v2/v3)

- ❌ Asumir un solo proyecto activo.
- ❌ Hardcodear tipos de nodo sin posibilidad de extensión.
- ❌ Acoplar UI de IA a un proveedor específico.
- ❌ Ignorar `generated_at` en `GraphData`.
- ❌ Usar strings mágicas como identificadores de error.

---

## 16. Acceptance Criteria de este estándar

- [ ] Todos los PRs pasan el checklist de revisión (§12).
- [ ] CI incluye los gates definidos en §11.
- [ ] No hay `.unwrap()`, `any`, ni `@ts-ignore` sin justificación en producción.
- [ ] Toda función pública de `engine/src/` tiene doc comment.
- [ ] Todo componente en `src/components/` tiene tipado explícito de props.
- [ ] Los tests alcanzan la cobertura mínima definida en §7.2.
- [ ] El equipo recibió este documento y confirmó entendimiento.
- [ ] El documento se revisa al cierre de cada versión mayor.

---

## Apéndices

### A. Onboarding checklist

- [ ] Leer `docs/MASTER_PROMPT_MVP_CERRADO.md`.
- [ ] Leer este documento completo.
- [ ] Leer `docs/GOBERNANZA_CONTRATOS_UI_BE.md`.
- [ ] Leer `docs/PLAN_CALIDAD_TESTS_BENCHMARKS.md`.
- [ ] Clonar repo y ejecutar tests.
- [ ] Hacer un PR de prueba aplicando el checklist §12.

### B. ¿Dónde pongo esto?

| Quiero crear… | Va en… |
|---|---|
| Un tipo de dato del dominio | `engine/src/models/` o `src/lib/types.ts` |
| Un comando Tauri | `engine/src/tauri_commands.rs` + `src/lib/tauri-api.ts` |
| Un componente visual | `src/components/<dominio>/` |
| Un componente genérico | `src/components/common/` |
| Un hook | `src/hooks/` |
| Estado global | `src/stores/` |
| Lógica de negocio pura (Rust) | `engine/src/graph/`, `engine/src/ai/context.rs` |
| Acceso a infraestructura | `engine/src/db/`, `engine/src/scanner/`, `engine/src/ai/anthropic.rs` |

### C. Comandos rápidos

```bash
# Rust
cargo fmt && cargo clippy -- -D warnings && cargo test

# TypeScript
npm run lint && npm run format && npm run test && npm run typecheck
```

---

*Documento vivo. Última actualización: 2026-05-31. Próxima revisión: cierre de v1.*
