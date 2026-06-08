# Design — hexagonal-architecture-wave-1-ports

## 1. Resumen ejecutivo

La ola 1 no persigue features nuevas: persigue **mover la lógica de borde y orquestación fuera de Tauri y fuera de la UI**, dejando contratos explícitos para persistencia, estado de app y consumo frontend.

El diseño se apoya en un principio simple:

- **presentación**: Tauri commands y componentes React;
- **aplicación**: services y hooks que orquestan casos de uso;
- **infraestructura**: repositorios SQLite, filesystem, parser/walker/graph builder, provider AI.

## 2. Línea base actual

### Backend

- `src-tauri/src/commands.rs` concentra 28 comandos y mezcla varias capas.
- `src-tauri/src/lib.rs` ya es un composition root parcial, pero hoy convive con instanciación directa en comandos.
- `engine/src/db/queries.rs` funciona como mega-repository.
- `engine/src/ai/` es el mejor antecedente local, pero todavía filtra adapters concretos.
- `engine/src/commands.rs` contiene helpers puros que conviene preservar en esta ola.

### Frontend

- `src/App.tsx` contiene orquestación que debería vivir en hooks.
- `src/lib/tauri-api.ts` mezcla bridge Tauri, mapeo de errores y wrappers de dominio.
- Algunos componentes todavía importan `tauri-api.ts` directo.

## 3. Decisiones arquitectónicas

### AD-1. Puertos canónicos de wave 1

Se adoptan estos contratos backend:

- `ScanRepository`
- `GraphRepository`
- `WorkspaceRepository`
- `AppStatePort`

`AppStatePort` cubre el estado transitorio del proceso Tauri (scan status, AI config, project root u otros equivalentes necesarios).

### AD-2. Ubicación mínima de puertos

Para reducir fricción en la primera ola, los puertos viven en `engine/src/ports.rs`.

Si la cantidad de traits crece en olas posteriores, puede migrarse a `engine/src/ports/` sin romper la dirección del diseño.

### AD-3. Servicios de aplicación canónicos

Se adoptan estos servicios backend:

- `ScanService`
- `GraphService`
- `WorkspaceService`
- `AnalysisService`

Y se mantiene el `AIService` existente como servicio del dominio AI.

### AD-4. `queries.rs` no se parte, pero sí se adapta

No se cambia la estructura interna de `engine/src/db/queries.rs` en wave 1.

Sí se permite una de estas dos variantes aditivas:

1. `impl <PortTrait> for ProjectRepository<'_>` dentro del mismo archivo; o
2. wrapper/adaptador fino en módulo vecino que delega en `ProjectRepository`.

La decisión concreta puede optimizarse por ergonomía de compilación, pero el criterio es constante: **sin rediseño del SQL ni split interno del mega-archivo**.

### AD-5. Composition root único

`src-tauri/src/lib.rs` pasa a ser el único lugar donde se crean adapters concretos y se cablean services.

Los commands dejan de crear o resolver directamente:

- `ProjectRepository`
- `FileWalker`
- `ParserRegistry`
- `PathResolver`
- `GraphBuilder`
- providers AI concretos

Cuando haga falta usar utilidades puras o existentes (`engine::commands`, resolvers, builders), se las inyecta o encapsula desde el composition root o a través de colaboradores explícitos, no desde el body del command.

### AD-6. Contrato de error sobre Tauri IPC

El payload de error estable será conceptualmente:

```json
{
  "code": "PROJECT_NOT_FOUND",
  "message": "Project not found: foo",
  "details": { "projectId": "foo" }
}
```

Pero sobre el canal Tauri se enviará como **JSON serializado en string** para no romper la realidad del IPC actual.

Frontend:

1. intenta `JSON.parse(err.message)`;
2. si encuentra `code` y `message`, mapea y devuelve `ApiError` tipado;
3. si no, cae al fallback legacy regex/string matching.

### AD-7. Catálogo backend vs `ErrorCode` frontend

No hace falta que el union `ErrorCode` frontend replique 1:1 el catálogo backend.

Sí hace falta una tabla explícita de mapeo, por ejemplo:

- `PROJECT_NOT_FOUND` -> `PATH_NOT_FOUND`
- `FILE_NOT_FOUND` -> `PATH_NOT_FOUND`
- `AI_UNAVAILABLE` -> `UNREACHABLE`
- `AI_RATE_LIMITED` -> `RATE_LIMITED`
- `AI_TOKEN_LIMIT` -> `TOKEN_LIMIT`
- `INVALID_API_KEY` -> `INVALID_KEY`
- `ACCESS_DENIED` -> `ACCESS_DENIED`
- `SCAN_TIMEOUT` -> `SCAN_TIMEOUT`
- `DATABASE` -> `INTERNAL`
- `INTERNAL` -> `INTERNAL`

`details` en frontend conserva forma estructurada: `Record<string, unknown> | undefined`.

### AD-8. `engine::commands` se preserva

Wave 1 no elimina `engine/src/commands.rs`.

Se lo trata como helper puro existente y reutilizable desde services donde sirva. La discusión sobre absorberlo o eliminarlo queda para una ola posterior, cuando el borde hexagonal ya exista de verdad.

## 4. Diseño objetivo por capa

### Backend

```text
Tauri command shim
  -> AppState (services ya cableados)
    -> ScanService / GraphService / WorkspaceService / AnalysisService / AIService
      -> Ports
        -> ProjectRepository wrappers/impls
        -> AppState adapter
        -> otros colaboradores explícitos
```

### Frontend

```text
App.tsx / components
  -> hooks de dominio
    -> services frontend
      -> tauri-api / invoke bridge
        -> Tauri backend
```

## 5. Alcance por slices

### PR-1 — Error contract

- `AppError` serializa JSON-string estable.
- `toApiError` parsea JSON primero y conserva fallback legacy.
- `ApiError.details` sigue siendo estructurado.

### PR-2 — Ports + adapters aditivos

- `engine/src/ports.rs`
- adaptación aditiva sobre `ProjectRepository`
- sin split interno de `queries.rs`

### PR-3 — ScanService

- extraer `scan_project`, `open_project_by_path`, `get_scan_status`
- mover composición correspondiente a root

### PR-4 — GraphService

- extraer `get_graph`, `get_node_details`, `get_node_outline`, `search` y afines de grafo

### PR-5 — WorkspaceService

- extraer comandos existentes de workspaces, snapshots, comments, health y C4
- esto **no agrega features**; refactoriza superficie ya existente

### PR-6 — AnalysisService + root cleanup

- extraer análisis/impact/insights/export si corresponde
- eliminar doble composición residual
- dejar `commands.rs` como shim delgado

### PR-7 — AI boundary cleanup

- ocultar adapters concretos del módulo AI
- preservar `AIService` y traits públicos necesarios

### PR-8 — Frontend services/hooks

- sacar imports directos de `tauri-api.ts` desde `App.tsx` y componentes
- centralizar carga/error/orquestación en hooks

## 6. Riesgos y mitigaciones

### R-1. Cambio de IPC error

Mitigación: PR atómico backend + frontend, más tests RED/GREEN en ambos lados.

### R-2. Explosión de traits

Mitigación: solo 4 puertos canónicos en wave 1. Nada de granularidad excesiva.

### R-3. Migración parcial de `commands.rs`

Mitigación: slices por dominio y composición root como punto único de verdad.

### R-4. `commands.rs` no baja lo suficiente

Mitigación: mover DTOs/helpers residuales a módulos separados si hace falta. Objetivo recomendado: <=300 LOC; techo aceptable para cierre de wave 1: <=350 LOC de código útil.

### R-5. Contaminación de v3 scope

Mitigación: solo refactor de comandos/superficie ya existente; cero features nuevas.

## 7. Evidencia esperada para verify

- tests existentes y nuevos en verde;
- diff dividido en slices revisables, con sizing real documentado antes de cerrar la estrategia final de entrega;
- `commands.rs` adelgazado;
- ausencia de imports directos a `tauri-api.ts` desde componentes;
- borde AI sin re-exports concretos.

## 8. Resultado esperado al final de wave 1

CodeAtlas queda con un borde hexagonal inicial real: los entrypoints de presentación dejan de hablar directo con infra, los casos de uso tienen services explícitos, el frontend consume hooks/services en vez de bridge crudo, y el resto del repo ya puede migrarse por slices sobre una base estable.
