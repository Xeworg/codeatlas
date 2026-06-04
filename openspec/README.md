# OpenSpec — CodeAtlas

Este directorio contiene artefactos SDD de CodeAtlas.

## Estado

- **Versiones archivadas:** v1-mvp-core (2026-06-01), v2-advanced-analysis (2026-06-01), robust-logging-observability (2026-06-04)
- **Versión activa:** v3 — SDD pendiente de iniciar
- Modo: **interactive**
- Store: **OpenSpec + Engram**
- Estrategia PR: **auto-forecast**
- Review budget: **400 líneas cambiadas**

## Cambio activo

Ninguno. Para iniciar v3, ejecutar `/sdd-init` o equivalente y avanzar las fases SDD hasta tasks antes de aplicar.

## Cambios archivados

- `openspec/changes/archive/2026-06-01-v1-mvp-core/` — v1 core, verificado y archivado
- `openspec/changes/archive/2026-06-01-v2-advanced-analysis/` — v2 advanced-analysis, verificado PASS con excepciones documentadas (NFR benchmarks scaffold, degraded-mode 4/8, App wiring T5.6 diferido)
- `openspec/changes/archive/2026-06-04-robust-logging-observability/` — logging robusto, normalización de errores frontend, tracing backend y logs dev por ejecución

## Fases SDD esperadas

1. `explore`
2. `proposal`
3. `spec`
4. `design`
5. `tasks`
6. `apply`
7. `verify`

## Contexto base para v3

- `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md` (scope v3: workspaces, snapshots, annotations, health timeline, C4, comparativas)
- `docs/ARQUITECTURA_DATOS_V2_V3.md` (schema v3: workspaces, snapshots, annotations, health_records)
- `docs/SEGURIDAD_PERMISOS_V3.md` (seguridad v3)
- `docs/CHANGELOG_CONTRATOS.md` (contratos v1/v2 implementados, v3 planificados)

## Convención de artefactos por cambio

Crear carpeta: `openspec/changes/<change-id>/`

Archivos mínimos:

- `proposal.md`
- `spec.md`
- `design.md`
- `tasks.md`
- `apply-progress.md`
- `verify-report.md` (al cierre)
