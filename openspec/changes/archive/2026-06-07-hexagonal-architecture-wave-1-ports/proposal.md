# Proposal — hexagonal-architecture-wave-1-ports

## Resumen

Este cambio planifica la **ola 1** de la migración de CodeAtlas hacia arquitectura hexagonal. El objetivo no es sumar features nuevas, sino **crear bordes claros** entre presentación, aplicación e infraestructura para que el resto del repo pueda migrarse por slices revisables.

La presión principal hoy está en tres hotspots:

1. `src-tauri/src/commands.rs` (~1526 LOC) mezcla comandos Tauri, acceso a infra, orquestación, errores y filesystem.
2. `engine/src/db/queries.rs` (~2383 LOC) concentra persistencia de múltiples dominios sin contratos útiles por encima.
3. `src/App.tsx` y `src/lib/tauri-api.ts` mezclan UI, orquestación y acceso directo al bridge Tauri.

## Objetivo

Establecer una base hexagonal para el repo mediante:

- contratos de puertos backend;
- servicios de aplicación por dominio;
- composition root único en Tauri;
- contrato de error tipado y estable sobre el canal IPC;
- capa frontend de services/hooks;
- limpieza del borde público del módulo AI.

**Regla central:** sin cambio de comportamiento visible.

## Alcance (Wave 1)

### Backend

- Definir puertos canónicos en Rust:
  - `ScanRepository`
  - `GraphRepository`
  - `WorkspaceRepository`
  - `AppStatePort`
- Extraer servicios de aplicación:
  - `ScanService`
  - `GraphService`
  - `WorkspaceService`
  - `AnalysisService`
- Mantener `engine::commands` como helpers puros existentes; no se elimina en esta ola.
- Permitir adaptación aditiva sobre `ProjectRepository` sin partir `queries.rs` internamente.
- Mover la composición a `src-tauri/src/lib.rs` para que los comandos dejen de instanciar infra.

### Frontend

- Introducir `src/services/` para encapsular `invoke()` y parseo de errores.
- Introducir hooks de dominio (`useProject`, `useGraph`, `useWorkspace`, `useAi` o equivalentes consistentes).
- Sacar imports directos de `tauri-api.ts` desde `App.tsx` y `src/components/**`.

### Contrato de errores

- Reemplazar el string opaco actual por un **payload JSON serializado como string** sobre el canal Tauri:
  - `{"code":"...","message":"...","details":{...}}`
- Mantener fallback legacy en frontend mientras dura la transición.

### AI

- Limpiar re-exports de adapters concretos desde `engine/src/ai/mod.rs`.
- Mantener `AIService` como superficie de consumo.

## Fuera de alcance

- No agregar features de producto nuevas.
- No partir `engine/src/db/queries.rs` en múltiples archivos ni rediseñar su SQL interno en esta ola.
- No cambiar schema de negocio salvo lo estrictamente necesario para el contrato de error.
- No incorporar nuevos providers AI.
- No reemplazar `engine::commands`; solo se preserva y reubica conceptualmente como helper puro.

## Decisiones guía

| Tema                 | Decisión                                                                                                          |
| -------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Nombre del cambio    | `hexagonal-architecture-wave-1-ports`                                                                             |
| Estrategia           | Definir el tamaño objetivo y si convienen PRs encadenados después de estimar mejor el alcance real                |
| `queries.rs`         | Sin split interno; se permiten impls/wrappers aditivos para adaptarlo a puertos                                   |
| Canal de error Tauri | JSON serializado como string, no objeto crudo IPC                                                                 |
| Hooks frontend       | Sin `invoke()` directo en componentes                                                                             |
| v3                   | Los comandos existentes de workspaces/snapshots/comments/health/C4 se refactorizan; no se agregan features nuevas |

## Riesgos

1. **Cambio de contrato IPC de errores**: si se hace mal, rompe el parseo frontend.
2. **Sobre-diseño de puertos**: demasiados traits finos puede inflar la complejidad.
3. **Migración parcial de `commands.rs`**: puede dejar doble composición y comportamientos inconsistentes.
4. **Carga de revisión**: el alcance total supera holgadamente un solo PR.
5. **Compatibilidad v3**: hay que refactorizar comandos existentes de ese dominio sin convertir esto en desarrollo de features.

## Estrategia de entrega

Trabajo encadenado, orientado a slices:

1. contrato de error;
2. puertos + adapters aditivos;
3. `ScanService`;
4. `GraphService`;
5. `WorkspaceService`;
6. `AnalysisService` + cleanup de composition root;
7. cleanup del borde AI;
8. services/hooks frontend.

## Criterios de aceptación

1. Todos los tests actuales siguen pasando.
2. `commands.rs` queda reducido a shim de presentación delgado.
3. Los comandos Tauri dejan de instanciar repositorios, walkers, parsers o resolvers directamente.
4. Existe una capa explícita de servicios de aplicación en backend.
5. El frontend deja de depender de parseo regex-only para errores backend.
6. `App.tsx` y `src/components/**` dejan de importar `tauri-api.ts` directamente.
7. El módulo AI no re-exporta adapters concretos en su superficie pública.
8. La estrategia de entrega (uno o varios PRs, y tamaño objetivo por PR) queda documentada recién cuando el sizing técnico sea más confiable.

## Áreas afectadas

- `engine/src/lib.rs`
- `engine/src/ports.rs`
- `engine/src/services/**`
- `engine/src/db/queries.rs` (solo adaptación aditiva)
- `engine/src/ai/**`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src/lib/tauri-api.ts`
- `src/lib/types.ts`
- `src/services/**`
- `src/hooks/**`
- `src/App.tsx`
- `src/components/**`

## Próximo paso

Convertir esta propuesta en specs, diseño y tasks de apply con slices estrictos de TDD y forecast de PRs encadenados.
