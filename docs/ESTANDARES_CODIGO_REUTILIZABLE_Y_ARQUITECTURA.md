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

Traducido a código real para este proyecto:

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
use engine::scanner::ProjectScanner; // ← vía trait

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

Para cumplir la regla sin que `application` conozca infraestructura concreta:

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

```rust
// engine/src/db/queries.rs (Infrastructure)
// IMPLEMENTA el trait

impl FileProvider for SqliteProjectRepo {
    fn list_files(&self, project_id: &str) -> Result<Vec<FileInfo>> {
        // consulta SQL real
    }
}
```

### 3.4 Inversión de dependencia con hooks (TypeScript)

```typescript
// ✅ CORRECTO: hook abstrae la fuente de datos
function useGraph(projectId: string) {
  const [data, setData] = useState<GraphData | null>(null);
  
  useEffect(() => {
    invokeGetGraph(projectId).then(setData);
  }, [projectId]);
  
  return data;
}

// El componente GraphView solo conoce el hook, no invoke.
```

---

## 4. Convenciones de código — Rust

### 4.1 Estilo y formato

```toml
# rustfmt.toml (raíz del workspace)
edition = "2021"
max_width = 100
tab_spaces = 4
use_small_heuristics = "Max"
```

- **Formatter automático:** `cargo fmt` obligatorio en pre-commit.
- **Linter:** `cargo clippy -- -D warnings`. PRs con warnings de clippy no mergean.

### 4.2 Nombrado

| Elemento | Convención | Ejemplo |
|---|---|---|
| Módulos / archivos | `snake_case` | `graph_builder.rs` |
| Structs / Enums / Traits | `PascalCase` | `GraphNode`, `NodeType` |
| Funciones / métodos | `snake_case` | `build_graph()`, `resolve_path()` |
| Constantes / estáticas | `SCREAMING_SNAKE_CASE` | `MAX_FILES`, `DEFAULT_TIMEOUT_MS` |
| Variables | `snake_case` | `file_count`, `scan_result` |
| Feature flags | `kebab-case` | `ai-anthropic` |

### 4.3 Organización de imports

```rust
// 1. std
use std::path::PathBuf;

// 2. Crates externos (alfabético)
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// 3. Crate interno (engine)
use engine::models::{FileInfo, GraphData};

// 4. Módulo actual (super/self)
use super::resolver::resolve_alias;
```

### 4.4 Manejo de Option y Result

- **Nunca usar `.unwrap()` en código de producción** salvo en tests o inicialización temprana que se demuestra infalible.
- Preferir `?` con `Result<T, AppError>`.
- Usar `Option<T>` solo cuando la ausencia es semánticamente válida (no para evitar manejar errores).

```rust
// ✅ CORRECTO
fn get_file(&self, id: &str) -> Result<FileInfo, AppError> {
    self.conn.query_row("SELECT ...", [id], |row| {
        Ok(FileInfo { /* ... */ })
    }).map_err(|e| AppError::Database(e.to_string()))
}

// ❌ INCORRECTO
fn get_file_unchecked(&self, id: &str) -> FileInfo {
    self.conn.query_row("SELECT ...", [id], |row| {
        Ok(FileInfo { /* ... */ })
    }).unwrap() // NUNCA en prod
}
```

### 4.5 Documentación

- Todo struct, trait, enum y función pública debe tener doc comment (`///`).
- Ejemplos de uso con `/// # Examples` cuando el uso no sea obvio.

```rust
/// Representa un archivo escaneado dentro de un proyecto.
///
/// # Examples
///
/// ```
/// let file = FileInfo::new("src/main.ts", "main.ts", "ts");
/// assert_eq!(file.extension, "ts");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Identificador único (UUID v4).
    pub id: String,
    /// Ruta relativa desde la raíz del proyecto.
    pub path: String,
    // ...
}
```

### 4.6 Error types

Usar `thiserror` para errores de dominio y aplicación:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Archivo no encontrado: {0}")]
    NotFound(String),
    
    #[error("Error de base de datos: {0}")]
    Database(String),
    
    #[error("Timeout de escaneo: {files_processed}/{total_files} archivos procesados")]
    ScanTimeout { files_processed: usize, total_files: usize },
    
    #[error("IA no disponible: {0}")]
    AIUnavailable(String),
    
    #[error("Error interno: {0}")]
    Internal(String),
}
```

---

## 5. Convenciones de código — TypeScript / React

### 5.1 Formato

```json
// .prettierrc
{
  "semi": true,
  "singleQuote": false,
  "trailingComma": "all",
  "printWidth": 100,
  "tabWidth": 2
}
```

- **Formatter:** Prettier (pre-commit hook).
- **Linter:** ESLint con `@typescript-eslint` strict. PRs con errores no mergean.

### 5.2 Nombrado

| Elemento | Convención | Ejemplo |
|---|---|---|
| Archivos de componente | `PascalCase.tsx` | `GraphView.tsx` |
| Archivos de hook / lib | `kebab-case.ts` o `camelCase.ts` | `useProject.ts`, `tauri-api.ts` |
| Componentes React | `PascalCase` | `function GraphView() {}` |
| Hooks | `use` + `PascalCase` | `useGraph()`, `useProject()` |
| Funciones / variables | `camelCase` | `buildLayout()`, `fileCount` |
| Tipos / Interfaces | `PascalCase` | `GraphData`, `FileInfo` |
| Constantes | `UPPER_SNAKE_CASE` | `MAX_VISIBLE_NODES` |
| Enums / Union types | `PascalCase` | `type ScanStatus = "idle" \| ...` |

### 5.3 Organización de imports

```typescript
// 1. React / librerías core
import React, { useState, useCallback } from "react";

// 2. Librerías externas (alfabético)
import { ReactFlow, Node, Edge } from "@xyflow/react";

// 3. Módulos del proyecto (alias @/ → src/)
import { GraphData } from "@/lib/types";
import { useGraph } from "@/hooks/useGraph";

// 4. Componentes locales (relativos)
import { GraphNode } from "./GraphNode";
import { MiniMap } from "./MiniMap";
```

### 5.4 Componentes React

- **Un componente = un archivo.** Si el archivo pasa de 200 líneas, extraer subcomponentes.
- **Props tipadas explícitamente** (nunca `any`):

```typescript
// ✅ CORRECTO
interface GraphViewProps {
  projectId: string;
  onNodeSelect?: (nodeId: string) => void;
}

function GraphView({ projectId, onNodeSelect }: GraphViewProps) {
  // ...
}

// ❌ INCORRECTO
function GraphView(props: any) { /* ... */ }
```

- **Estado local solo cuando el estado no necesita compartirse.** Si dos componentes en ramas distintas del árbol necesitan el mismo dato, va a Zustand.
- **Custom hooks** para lógica reutilizable. Si una lógica se repite en 2+ componentes, extraer hook.

### 5.5 Zustand stores

```typescript
// ✅ Patrón recomendado: store con acciones atómicas
interface ProjectStore {
  // Estado
  status: ScanStatus;
  files: FileInfo[];
  selectedNodeId: string | null;
  
  // Acciones
  startScan: (path: string) => Promise<void>;
  selectNode: (nodeId: string) => void;
  resetProject: () => void;
}

// Separar selectores para evitar re-renders innecesarios
export const useScanStatus = () => useProjectStore((s) => s.status);
export const useSelectedNodeId = () => useProjectStore((s) => s.selectedNodeId);
```

---

## 6. Estrategia de manejo de errores

### 6.1 Jerarquía de errores

```
AppError (Rust)          → serializado a JSON → recibido por frontend
  ├─ NotFound
  ├─ Database
  ├─ ScanTimeout
  ├─ AIUnavailable
  ├─ InvalidApiKey
  ├─ RateLimited
  └─ Internal
```

### 6.2 Contrato de error en API Tauri

```typescript
// Forma canónica de error desde Rust hacia frontend
interface ApiError {
  code: ErrorCode;
  message: string;       // legible para el usuario
  details?: Record<string, unknown>;  // opcional, para debugging
}

type ErrorCode =
  | "PATH_NOT_FOUND"
  | "ACCESS_DENIED"
  | "SCAN_TIMEOUT"
  | "INVALID_KEY"
  | "UNREACHABLE"
  | "RATE_LIMITED"
  | "TOKEN_LIMIT"
  | "INTERNAL";
```

### 6.3 Manejo en Rust

```rust
// En tauri_commands.rs: todo comando captura errores y los convierte
#[tauri::command]
fn scan_project(path: String) -> Result<ScanResult, AppError> {
    // Tauri serializa AppError automáticamente si implementa serde::Serialize
    let scanner = ProjectScanner::new();
    scanner.scan(&path)
}
```

### 6.4 Manejo en TypeScript

```typescript
// En hooks/: siempre try/catch con estado de error
async function scanProject(path: string) {
  try {
    setStatus("scanning");
    const result = await invoke<ScanResult>("scan_project", { path });
    setStatus("building_graph");
  } catch (err) {
    const apiError = err as ApiError;
    setError(apiError.message);
    setStatus("error");
  }
}
```

### 6.5 UX de errores

| Error | Mensaje al usuario | Acción sugerida |
|---|---|---|
| `PATH_NOT_FOUND` | "No se encontró la carpeta seleccionada." | "Seleccionar otra carpeta" |
| `SCAN_TIMEOUT` | "El escaneo tardó más de lo esperado." | "Reintentar" |
| `INVALID_KEY` | "API key inválida." | "Ir a Settings" |
| `UNREACHABLE` | "No se puede conectar al proveedor de IA." | "Verificar conexión" |
| `RATE_LIMITED` | "Límite de consultas alcanzado." | "Esperar unos minutos" |
| `INTERNAL` | "Error inesperado." | "Reiniciar app" + logs |

---

## 7. Estrategia de testing

### 7.1 Pirámide de tests

```
         ╱ E2E ╲          ← Manual checklist v1, automatizado v2
        ╱─────────╲
       ╱Integration╲       ← DB + fixtures (Rust), store + mock invoke (TS)
      ╱───────────────╲
     ╱   Unit Tests    ╲    ← funciones puras, componentes aislados
    ╱─────────────────────╲
```

### 7.2 Distribución

| Nivel | Herramienta | Meta v1 | Tiempo CI |
|---|---|---|---|
| Unitarios Rust | `cargo test` | ≥ 80% cobertura | < 30s |
| Unitarios TS | `vitest` | ≥ 75% cobertura | < 20s |
| Integración Rust | `cargo test -- --ignored` | fixtures clave | < 60s |
| Integración TS | `vitest` + mock `invoke` | stores/hooks clave | < 30s |
| Contratos | `cargo test -- contracts` | todos los comandos Tauri | < 15s |
| E2E | Checklist manual | 10 flujos | manual |
| Benchmarks | `cargo bench` | scan + build graph | informativo |

### 7.3 Qué testear (no negociable)

| Si existe… | Debe tener… |
|---|---|
| Función pública en Rust | ≥ 1 test unitario |
| Hook de React | ≥ 1 test con `renderHook` |
| Endpoint Tauri | ≥ 1 test de contrato (snapshot) |
| Store de Zustand | ≥ 1 test por acción y por estado de error |
| Componente con loading/error/empty | ≥ 1 test por estado |

### 7.4 Nombrado de tests

```rust
// Rust: describe el comportamiento esperado
#[test]
fn builder_returns_correct_graph_for_project_with_three_files() { }
#[test]
fn resolver_returns_error_when_alias_not_found_in_tsconfig() { }
```

```typescript
// TypeScript: describe el comportamiento esperado
it("returns idle status when no project is loaded", () => { });
it("shows error message when scan fails", () => { });
```

### 7.5 Fixtures

- Fixtures de prueba en `fixtures/`: proyectos TS/JS reales pequeños.
- No mockear Tree-sitter en tests de integración; usar fixtures reales.
- Mockear solo HTTP (IA) y filesystem en tests unitarios.

---

## 8. Disciplina de contratos API

### 8.1 Source of truth

Los tipos en `src/lib/types.ts` y `engine/src/models/` son **el contrato canónico**. Cualquier cambio en un lado debe reflejarse en el otro en el **mismo PR**.

### 8.2 Checklist de sincronización (por PR que toca contratos)

- [ ] `types.ts`: interface actualizada.
- [ ] `models/*.rs`: struct actualizada con `#[derive(Serialize, Deserialize)]`.
- [ ] `#[serde(rename_all = "camelCase")]` presente en todas las structs serializadas.
- [ ] Campos nuevos: `Option<T>` en Rust, `field?` en TS.
- [ ] `tauri-api.ts`: firma de invoke coincide con comando Rust.
- [ ] Contract test snapshot actualizado.
- [ ] `docs/CHANGELOG_CONTRATOS.md` actualizado.
- [ ] Breaking changes documentados con plan de deprecación.

### 8.3 Versionado de contratos

- Contract version (`v1`) ≠ App version (`v1.2.3`).
- Contract version solo cambia si hay breaking changes irreversibles.
- Breaking change → deprecar en `N.minor` → remover en `N+1.major`.

### 8.4 Regla de oro

> **Un cambio de tipo DEBE reflejarse en ambos lados (TS + Rust) en el MISMO PR.**  
> No se mergea frontend sin backend ni backend sin frontend cuando tocan el contrato.

---

## 9. Componentes y servicios reutilizables

### 9.1 Definición de "reutilizable"

Un componente o servicio es **reutilizable** si:
- Está en `src/components/common/` (UI) o es un trait público en `engine/src/` (Rust).
- No tiene estado específico de una feature (ej: no sabe si es "el proyecto A" o "el B").
- Recibe todo lo que necesita vía props/parámetros.
- Está documentado con JSDoc o doc comment.
- Tiene al menos 1 test unitario.

### 9.2 Componentes UI reutilizables (v1)

| Componente | Ubicación | Props mínimas |
|---|---|---|
| `<Button>` | `src/components/common/Button.tsx` | `variant`, `size`, `disabled`, `onClick`, `children` |
| `<Modal>` | `src/components/common/Modal.tsx` | `open`, `onClose`, `title`, `children` |
| `<Spinner>` | `src/components/common/Spinner.tsx` | `size` |
| `<MarkdownView>` | `src/components/common/MarkdownView.tsx` | `content` (string) |
| `<EmptyState>` | `src/components/common/EmptyState.tsx` | `icon`, `title`, `description`, `action?` |
| `<ErrorState>` | `src/components/common/ErrorState.tsx` | `message`, `onRetry?`, `actionLabel?` |
| `<Skeleton>` | `src/components/common/Skeleton.tsx` | `width`, `height`, `variant` |

### 9.3 Servicios Rust reutilizables (v1)

| Trait | Ubicación | Propósito |
|---|---|---|
| `FileProvider` | `engine/src/graph/builder.rs` | Proveer archivos e imports para construir grafo |
| `ProjectRepository` | `engine/src/db/queries.rs` | CRUD de proyectos, archivos, símbolos |
| `AIProvider` | `engine/src/ai/provider.rs` | Abstracción de proveedor IA |
| `ContextBuilder` | `engine/src/ai/context.rs` | Construir contexto para prompts |

### 9.4 Regla de extracción

> Si una lógica se repite en **2 o más lugares**, se extrae a componente/hook/trait/función reutilizable **antes del merge**, no después.

---

## 10. Anti-patrones prohibidos

### 10.1 Rust

| Anti-patrón | Por qué es daño | Qué hacer en su lugar |
|---|---|---|
| `.unwrap()` en producción | Pánico silencioso que crashea la app | `?` con `Result<T, AppError>` |
| `String` como tipo de error | Imposible hacer match en el frontend | `enum AppError` con `thiserror` |
| Struct de dominio con dependencias externas | Rompe Clean Architecture, hace tests imposibles | Solo `serde` en models |
| Módulo `scanner/` que importa `graph/` | Acoplamiento circular | Scanner produce datos crudos; graph los consume |
| `panic!()` en código de análisis | Mata el proceso Tauri | `Result::Err` con mensaje descriptivo |
| Ignorar `Result` con `let _ =` | Bugs silenciosos | Manejar explícitamente o loguear |
| Funciones de 100+ líneas | Ilegibles, imposibles de testear | Extraer funciones privadas de ≤ 50 líneas |

### 10.2 TypeScript / React

| Anti-patrón | Por qué es daño | Qué hacer en su lugar |
|---|---|---|
| `any` | Anula el type system | Tipo explícito, `unknown` si es necesario |
| `invoke()` directo en componente | Acopla UI a Tauri, imposible de mockear | Hook o store |
| Estado duplicado en 2 stores | Fuente de bugs de sincronización | Una sola fuente de verdad |
| `useEffect` con lógica de negocio compleja | Mezcla efectos con lógica, difícil de testear | Extraer a hook o función pura |
| Props drilling > 3 niveles | Código frágil, difícil de refactorizar | Zustand o composición |
| Componente de 300+ líneas | Ilegible, imposible de revisar | Extraer subcomponentes |
| `// @ts-ignore` | Oculta errores reales | Arreglar el tipo o usar `as` con comentario |
| `setTimeout` para sincronización | Race conditions, flaky tests | Estado explícito o useEffect con dependencias |

### 10.3 General

| Anti-patrón | Por qué es daño | Qué hacer en su lugar |
|---|---|---|
| PR de 800+ líneas | Imposible de revisar bien | Dividir en commits atómicos o chained PRs |
| Mergear sin tests | Deuda técnica inmediata | ≥ 1 test por feature |
| "Lo arreglo después" | Nunca se arregla | Issue en backlog con prioridad |
| Features de v2 coladas en PR de v1 | Scope creep, retrasa MVP | Mover a rama separada o backlog |
| Hardcodear valores que deberían ser configurables | Cambio requiere deploy | Constante o settings |

---

## 11. CI y gates de calidad

### 11.1 Pipeline mínimo (GitHub Actions)

```yaml
name: CI

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: npm run lint

  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
      - run: cargo test -- --ignored  # integration
      - run: npm run test

  contracts:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test -- contracts

  typecheck:
    runs-on: ubuntu-latest
    steps:
      - run: npm run typecheck

  benchmarks:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - run: cargo bench
```

### 11.2 Gates que bloquean merge

| Gate | ¿Bloquea? | ¿Cuándo corre? |
|---|---|---|
| `cargo fmt --check` | ✅ | Todo PR |
| `cargo clippy -- -D warnings` | ✅ | Todo PR |
| `npm run lint` | ✅ | Todo PR |
| `cargo test` + `npm run test` | ✅ | Todo PR |
| Contract tests | ✅ | PRs que tocan modelos o tipos |
| `npm run typecheck` | ✅ | Todo PR |
| Benchmarks | ❌ (informativo) | Solo main |

---

## 12. Checklist de revisión de PR

### 12.1 Para el autor (antes de pedir review)

- [ ] Código compila (`cargo build` y `npm run build`).
- [ ] Tests pasan localmente (`cargo test` y `npm run test`).
- [ ] Linter limpio (`cargo clippy` y `npm run lint`).
- [ ] Formato aplicado (`cargo fmt` y `prettier`).
- [ ] Tipos sincronizados TS ↔ Rust si toca contratos.
- [ ] Nuevos endpoints/funciones públicas tienen doc comments.
- [ ] Nuevos componentes/hooks/traits tienen ≥ 1 test.
- [ ] No se introdujeron dependencias circulares.
- [ ] No hay `.unwrap()`, `any`, `@ts-ignore` sin justificación escrita.
- [ ] PR ≤ 400 líneas (o justificado y aprobado por TL).
- [ ] Si es breaking change: documentado en CHANGELOG.

### 12.2 Para el reviewer

- [ ] ¿La lógica de negocio está en la capa correcta (no en UI ni DB)?
- [ ] ¿Hay tests para el happy path y al menos 1 edge case?
- [ ] ¿Los errores se manejan con el tipo correcto (`AppError`, `ApiError`)?
- [ ] ¿Los estados de UI (loading, empty, error) están cubiertos?
- [ ] ¿El contrato de API es backward-compatible? Si no, ¿está documentado?
- [ ] ¿Se respetan los límites de módulo? (ej: models/ no importa db/)
- [ ] ¿El código es reutilizable o tiene justificación para ser específico?
- [ ] ¿Se introdujo scope creep de v2/v3? → rechazar o mover a feature branch.
- [ ] ¿Los nombres son descriptivos y consistentes con las convenciones?
- [ ] ¿Hay código duplicado que debería extraerse?

---

## 13. Política de refactoring

### 13.1 Cuándo refactorizar

| Situación | Acción |
|---|---|
| Misma lógica aparece en 2+ lugares | Extraer a función/hook/trait reutilizable. |
| Función > 50 líneas (Rust) o componente > 200 líneas (React) | Extraer sub-funciones/componentes. |
| Nombre de variable/función no describe lo que hace | Renombrar (refactor seguro con IDE). |
| Código comentado o dead code | Eliminar. Git guarda el historial. |
| Deuda técnica identificada en review | Issue etiquetado `refactor` con prioridad. |

### 13.2 Reglas de refactoring

1. **Refactoring y feature en PRs separados.** Nunca mezclar "mejoré la estructura" con "agregué feature X" en el mismo PR.
2. **Refactoring no cambia comportamiento.** Si un refactor requiere cambiar tests de comportamiento, no es refactor, es reimplementación.
3. **Refactoring grandes (> 400 líneas) → chained PRs o aprobación explícita de TL.**
4. **Siempre con tests verdes antes y después.**

### 13.3 Boy scout rule

> Dejá el código un poco mejor de como lo encontraste.  
> Si ves un `unwrap()` viejo, un `any` suelto, o un componente sin test al tocarlo para otra cosa → arreglalo en el mismo PR (si es ≤ 20 líneas extra). Si es más grande → issue separado.

---

## 14. Política de rollout de cambios

### 14.1 Branches

```
main          ← producción (siempre verde, siempre deployable)
├─ feat/*     ← features nuevas
├─ fix/*      ← bugs
├─ refactor/* ← refactors
└─ docs/*     ← documentación
```

### 14.2 Flujo de cambio

```
1. Crear branch desde main: feat/scan-project
2. Implementar con tests
3. PR → CI verde → review → aprobación
4. Merge a main (squash para features, merge commit para fixes)
5. main se considera release candidate
```

### 14.3 Chained PRs (cuando aplica)

Si una feature requiere > 400 líneas:

1. Dividir en PRs encadenados (cada uno compila y pasa tests).
2. PR base → PR siguiente (stacked diffs).
3. Cada PR se revisa y mergea en orden.
4. Referencia: skill `chained-pr`.

### 14.4 Rollback

- Si un merge a main rompe CI o introduce bug P0 → revertir el PR completo.
- No se hace "hotfix sobre hotfix". Revertir primero, analizar después.

---

## 15. Evolución forward-compat (v2/v3)

### 15.1 Qué protegemos ahora para no romper después

| Lo que hacemos en v1 | Cómo facilita v2/v3 |
|---|---|
| `AIProvider` como trait | Agregar nuevos proveedores sin tocar dominio |
| `FileProvider` como trait | Cambiar fuente de datos (DB → archivo → red) sin tocar graph builder |
| `NodeType` con variantes desde v1 | Agregar `route`, `middleware`, etc. sin breaking change si usamos enum abierto |
| `GraphData` con `generated_at` | Snapshots comparativos en v3 |
| `AppError` como enum | Agregar variantes sin romper matches existentes |
| Stores Zustand atómicos | Agregar nuevas stores en v2/v3 sin refactorizar stores existentes |
| SQLite con `content_hash` | Escaneo incremental en v2 sin cambiar schema |
| Contratos versionados con deprecación | Remover campos en v2 sin breaking change repentino |

### 15.2 Qué NO hacemos en v1 (para no hipotecar v2/v3)

- ❌ Asumir un solo proyecto activo (preparar stores para array de proyectos).
- ❌ Hardcodear tipos de nodo sin posibilidad de extensión.
- ❌ Acoplar UI de IA a un proveedor específico (usar abstracción desde v1).
- ❌ Ignorar `generated_at` en `GraphData` (v3 necesita timestamps para snapshots).
- ❌ Usar strings mágicas como identificadores de error (usar `enum AppError`).

---

## 16. Acceptance Criteria de este estándar

Este documento se considera **adoptado** cuando:

- [ ] Todos los PRs de Sprint 0 en adelante pasan el checklist de revisión (§12).
- [ ] CI incluye los gates definidos en §11.
- [ ] No hay `.unwrap()`, `any`, ni `@ts-ignore` sin justificación explícita en el código de producción.
- [ ] Toda función pública de `engine/src/` tiene doc comment.
- [ ] Todo componente en `src/components/` tiene tipado explícito de props.
- [ ] Los tests alcanzan la cobertura mínima definida en §7.2.
- [ ] El equipo recibió este documento y confirmó entendimiento (onboarding-checklist).
- [ ] El documento se revisa y actualiza al cierre de cada versión mayor.

---

## Apéndice A: Onboarding checklist (para nuevo desarrollador)

- [ ] Leer `docs/MASTER_PROMPT_MVP_CERRADO.md` (visión y alcance).
- [ ] Leer este documento completo (§1–§16).
- [ ] Leer `docs/GOBERNANZA_CONTRATOS_UI_BE.md`.
- [ ] Leer `docs/PLAN_CALIDAD_TESTS_BENCHMARKS.md`.
- [ ] Clonar repo y ejecutar `cargo test` + `npm run test` (deben pasar).
- [ ] Hacer un PR de prueba (docs o fix trivial) aplicando el checklist §12.

## Apéndice B: Referencias rápidas

### B.1 ¿Dónde pongo esto?

| Quiero crear… | Va en… |
|---|---|
| Un tipo de dato del dominio | `engine/src/models/` o `src/lib/types.ts` |
| Un comando Tauri | `engine/src/tauri_commands.rs` (handler) + `src/lib/tauri-api.ts` (wrapper tipado) |
| Un componente visual | `src/components/<domain>/` |
| Un componente genérico | `src/components/common/` |
| Un hook | `src/hooks/` |
| Estado global | `src/stores/` |
| Lógica de negocio pura (Rust) | `engine/src/graph/`, `engine/src/ai/context.rs` |
| Acceso a infraestructura (DB, FS, HTTP) | `engine/src/db/`, `engine/src/scanner/`, `engine/src/ai/anthropic.rs` |

### B.2 Comandos rápidos

```bash
# Rust
cargo fmt                        # formatear
cargo clippy -- -D warnings      # lintear
cargo test                       # tests unitarios
cargo test -- --ignored           # tests integración
cargo bench                      # benchmarks

# TypeScript
npm run lint                     # lintear
npm run format                   # formatear
npm run test                     # tests
npm run typecheck                # verificar tipos
npm run build                    # build de producción
```

---

*Documento vivo. Última actualización: 2026-05-31. Próxima revisión: cierre de v1 (GA).*
