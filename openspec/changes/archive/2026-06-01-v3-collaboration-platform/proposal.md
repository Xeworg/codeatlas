# Proposal — v3-collaboration-platform

## Resumen

Este cambio inicia formalmente **v3** tras el archive de v2 para evolucionar CodeAtlas de herramienta individual a plataforma colaborativa multi-proyecto con capa ejecutiva.

La proposal incorpora explícitamente las excepciones heredadas de v2 y define su tratamiento dentro de v3 para evitar deuda oculta en el arranque.

## Intento / Objetivo v3

Habilitar que equipos colaboren sobre arquitectura con:

1. Contexto multi-proyecto (workspaces)
2. Snapshots compartibles y comparables
3. Comentarios/anotaciones sobre nodos
4. Dashboard ejecutivo con evolución temporal (health timeline)
5. Vistas C4 asistidas (Nivel 1/2) y comparativas entre snapshots

## Alcance (In Scope)

### A) Carry-over obligatorio de v2 (fase de entrada v3)

Tomado de `openspec/changes/archive/2026-06-01-v2-advanced-analysis/archive-report.md`:

1. **NFR benchmarks con fixture real** (1000+ archivos) y mediciones contra umbrales.
2. **Degraded-mode frontend/IA pendiente**: cubrir 4 escenarios faltantes (PNG fallback mock, contract mismatch, IA no configurada, IA timeout).
3. **App-level wiring T5.6**: integrar en `App.tsx` componentes analíticos ya wire-ready.

**Decisión de manejo:** estos 3 items entran en v3 como **gates de hardening temprano** antes de cerrar H1.

### B) Núcleo funcional v3

Basado en `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md` y `docs/ARQUITECTURA_DATOS_V2_V3.md`:

- Multi-proyecto / workspaces
- Snapshots de arquitectura
- Comentarios/anotaciones
- Historial temporal de health score
- Servicios de resumen ejecutivo y comparativas
- UI ejecutiva: C4 asistido (L1/L2) y diff entre snapshots

### C) Contratos y datos v3 (a detallar en spec/design)

- Contratos planificados: `Snapshot`, `Comment`, `SharedView`, `HealthScoreTimeline`, `ExecutiveArchitectureSummary`
- Migraciones planificadas: `004_workspace_and_snapshots.sql`, `005_collaboration_annotations.sql`, `006_health_timeline.sql`

## Non-Goals (fuera de alcance de esta proposal)

- Backend cloud multiusuario en tiempo real (v3 mantiene enfoque local-first documentado)
- Resolver sincronización distribuida/CRDT completa
- Reescritura total de v1/v2 contratos existentes
- Nuevas features v4 no descritas en plan maestro

## Riesgos clave

1. **Scope inflation** por mezclar hardening v2 + core v3 en paralelo.
2. **Complejidad de datos** por crecimiento de tablas snapshots/health_records.
3. **Consistencia de contratos UI/BE** en features colaborativas y ejecutivas.
4. **Rendimiento** en comparativas y dashboard con datasets medianos/grandes.
5. **Decisiones abiertas de colaboración** (límites local-first/sync) pueden afectar diseño.

## Mitigación inicial

- Tratar carry-over v2 como gates verificables tempranos.
- Particionar v3 por hitos H1/H2/H3 (multi-proyecto, colaboración básica, panel ejecutivo).
- Mantener migraciones aditivas y rollback vía backup SQLite.
- Exigir pruebas de integración UI-BE por hito.

## Rollback

Si algún slice rompe estabilidad de v2 o contratos base:

- Revert del slice completo
- Restauración de backup DB si hay migraciones aplicadas
- Replanificación sin ampliar scope hasta recuperar baseline PASS

## Criterios de éxito (proposal-level)

1. V3 arranca formalmente con cambio activo y alcance explícito.
2. Las 3 excepciones de v2 quedan asignadas como trabajo obligatorio de v3 temprano.
3. Hitos v3 quedan listos para bajar a requisitos testables en `spec.md`.
4. Se preserva trazabilidad con evidencia de docs y archive-report.

## Áreas afectadas (plan)

- `openspec/changes/v3-collaboration-platform/*`
- `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md`
- `docs/ARQUITECTURA_DATOS_V2_V3.md`
- `docs/CHANGELOG_CONTRATOS.md`
- `openspec/changes/archive/2026-06-01-v2-advanced-analysis/archive-report.md`

## Referencias de evidencia

- `openspec/changes/archive/2026-06-01-v2-advanced-analysis/archive-report.md`
- `openspec/changes/archive/2026-06-01-v2-advanced-analysis/verify-report.md`
- `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md`
- `docs/ARQUITECTURA_DATOS_V2_V3.md`
- `docs/CHANGELOG_CONTRATOS.md`
