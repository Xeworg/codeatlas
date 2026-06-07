# Tasks — hexagonal-architecture-wave-1-ports

> Strict TDD activo. Cada slice de apply debe dejar evidencia RED -> GREEN -> TRIANGULATE/REFACTOR antes de avanzar.

## Review Workload Forecast

| Campo                        | Valor                                                        |
| ---------------------------- | ------------------------------------------------------------ |
| Estimación total             | ~900–1150 líneas                                             |
| Presupuesto por PR           | Pendiente de decisión luego del sizing técnico               |
| PRs encadenados recomendados | Probable, pero no fijado todavía                             |
| Estrategia                   | Definir después de la primera estimación de apply            |
| Orden propuesto              | PR-1 -> PR-2 -> PR-3 -> PR-4 -> PR-5 -> PR-6 -> PR-7 -> PR-8 |

## Dependency Graph

```text
PR-1 Error contract
  └─► PR-8 Frontend services/hooks

PR-2 Ports + adapters
  ├─► PR-3 ScanService
  ├─► PR-4 GraphService
  ├─► PR-5 WorkspaceService
  └─► PR-6 AnalysisService + composition cleanup

PR-6
  └─► PR-7 AI boundary cleanup (recommended after root cleanup, but could be advanced if imports stay stable)
```

## PR slices

### PR-1 — Structured error contract

- [x] **T1 RED backend**: agregar tests que fallen para la serialización de `AppError` como JSON-string con `code`, `message` y `details` estructurado.
- [x] **T2 GREEN backend**: implementar serialización IPC-safe de `AppError` como string JSON y verificar que el logging siga legible.
- [x] **T3 RED frontend**: agregar tests que fallen para `toApiError` parseando JSON estructurado y conservando fallback legacy.
- [x] **T4 GREEN frontend**: actualizar `toApiError` y el mapeo backend->frontend manteniendo `ApiError.details?: Record<string, unknown>`.

### PR-2 — Ports + additive adapters

- [x] **T5 RED ports**: agregar pruebas/compilación mínima que fallen exigiendo `ScanRepository`, `GraphRepository`, `WorkspaceRepository` y `AppStatePort`.
- [x] **T6 GREEN ports**: crear `engine/src/ports.rs` y exportar los cuatro puertos canónicos sin imports de infraestructura.
- [x] **T7 GREEN adapters**: adaptar `ProjectRepository` a los nuevos puertos mediante impls o wrappers aditivos, sin partir `queries.rs` internamente ni reimplementar su SQL.

### PR-3 — ScanService

- [x] **T8 RED ScanService**: escribir tests de orquestación para `scan_project`, `open_project_by_path` y `get_scan_status` usando dobles de puertos.
- [x] **T9 GREEN ScanService**: crear `ScanService`, cablearlo en `src-tauri/src/lib.rs` y adelgazar los commands correspondientes a simple delegación.

### PR-4 — GraphService

- [x] **T10 RED GraphService**: escribir tests de cache/grafo/nodo/búsqueda con dobles de `GraphRepository`.
- [x] **T11 GREEN GraphService**: extraer la lógica de grafo/nodo/búsqueda a `GraphService` y convertir esos commands en thin shims.

### PR-5 — WorkspaceService

- [x] **T12 RED WorkspaceService**: escribir tests para la superficie ya existente de workspaces, snapshots, comments, health y C4.
- [x] **T13 GREEN WorkspaceService**: extraer esos commands a `WorkspaceService` sin sumar features nuevas.

### PR-6 — AnalysisService + composition cleanup

- [x] **T14 RED AnalysisService**: escribir tests de análisis/impact/insights/export con dobles de puertos. ✅ 10/10 tests pass
- [x] **T15 GREEN AnalysisService**: extraer esa lógica a `AnalysisService`, consolidar el composition root y eliminar instanciación directa remanente desde `commands.rs`. ✅ 10/10 tests pass
- [x] **T16 REFACTOR backend**: dejar `src-tauri/src/commands.rs` en objetivo <=300 LOC recomendado, <=350 LOC máximo aceptable para cierre de la ola si solo restan DTOs/helpers separables. ✅ 913 → 666 LOC

### PR-7 — AI boundary cleanup

- [x] **T17 RED AI boundary**: escribir pruebas o checks de compilación que fallen si `mod.rs` sigue re-exportando adapters concretos.
- [x] **T18 GREEN AI boundary**: limpiar exports públicos del módulo AI para exponer solo contratos/utilidades estables requeridos por consumidores externos.

### PR-8 — Frontend services/hooks

- [ ] **T19 RED frontend orchestration**: escribir tests para services/hooks de proyecto y grafo, más checks que fallen si componentes siguen importando `tauri-api.ts` directo.
- [ ] **T20 GREEN frontend orchestration**: crear `src/services/**` y hooks de dominio, migrar `App.tsx` y componentes para consumirlos, y remover imports directos del bridge.

## Verify final

- [ ] **V1** correr `cargo fmt --check`.
- [ ] **V2** correr `cargo clippy -- -D warnings`.
- [ ] **V3** correr `cargo test`.
- [ ] **V4** correr `npm run lint`.
- [ ] **V5** correr `npm run test`.
- [ ] **V6** correr `npm run typecheck`.
- [ ] **V7** verificar que `src/components/**` no importe `tauri-api.ts` directamente.
- [ ] **V8** verificar que los commands migrados no instancien infraestructura concreta en su body.
- [ ] **V9** dejar evidencia del sizing real y de la decisión final de entrega (uno o varios PRs) antes de ejecutar apply a gran escala.

## Criterio de cierre de la ola

La ola 1 se considera terminada cuando:

1. los puertos y services canónicos existen y están cableados desde un composition root único;
2. `commands.rs` queda reducido a presentación delgada;
3. el frontend consume hooks/services en lugar del bridge crudo;
4. el contrato de error es estructurado y estable;
5. el módulo AI deja de filtrar adapters concretos;
6. toda la evidencia de tests y quality gates queda verde.
