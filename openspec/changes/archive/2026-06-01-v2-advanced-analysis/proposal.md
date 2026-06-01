# Proposal — v2-advanced-analysis

## Resumen

Este cambio define **v2 de CodeAtlas** para pasar de comprensión básica (v1) a análisis técnico accionable: detección de arquitectura con evidencia, análisis de impacto, insights de grafo (ciclos/hotspots), exportes y mejoras de UX analítica. Incluye además una adición aprobada de bajo riesgo: **base i18n** (catálogo `es.json` + helper `t('key')`, sin selector de idioma).

## Intento / Objetivo v2

Permitir que un usuario técnico responda en minutos:

1. ¿Qué arquitectura parece tener este proyecto y con qué confianza?
2. ¿Qué se ve afectado si cambio este nodo/archivo?
3. ¿Dónde están ciclos y hotspots principales?
4. ¿Cómo comparto evidencia (JSON/PNG) del análisis?

## Alcance (In Scope)

### Backend

- Detección de arquitectura (MVC/Layered/Clean/Hexagonal/unknown) con score y evidencia.
- Aristas avanzadas progresivas (`usage`/`call`) sobre base v1.
- Motor de impacto de cambios (`ImpactAnalysisResult`).
- Cálculo de insights (`GraphInsights`): ciclos, hotspots, acoplamiento promedio, densidad.
- Endpoint de exportes (JSON/PNG) y payload tipado.
- Migración v2 aditiva de DB (`003_architecture_and_insights.sql`) sin romper datos v1.

### Frontend

- Modos/vistas analíticas v2 y filtros persistentes.
- Vista de ciclos y hotspots con navegación usable.
- Tarjeta de arquitectura detectada con evidencia y nivel de confianza.
- Flujo de aplicación (beta) como tipo adicional de diagrama.
- Export de vista/resultado (JSON/PNG).

### Foundation i18n (adición aprobada)

- Extraer textos UI a `locales/es.json`.
- Introducir helper `t('key')` y migrar strings visibles.
- Mantener idioma runtime fijo en español en v2 (sin selector).
- Dejar estructura lista para `locales/en.json` en versiones futuras.

## Límites de Alcance (Out of Scope / Non-Goals)

- Multi-proyecto/workspaces (v3).
- Snapshots colaborativos, comentarios/anotaciones (v3).
- Dashboard ejecutivo y health timeline (v3).
- Cambios breaking de contratos v1 sin versionado.
- Selector de idioma en UI (solo foundation i18n en v2).
- Virtualización avanzada >1000 nodos (deuda evaluable fuera de este corte si compromete salida v2).

## Métricas de Éxito v2 (propuestas para aprobar en Spec)

- Detección de arquitectura (5000 archivos): **<3s**.
- Análisis de impacto por nodo: **<5s**.
- Generación de insights (2000 nodos): **<2s**.
- Export JSON: **<5s**; export PNG: **<10s**.
- Respuesta UI a cambio de filtros: **<200ms** percibidos.
- Concurrencia WAL en lectura: **0 deadlocks** bajo 10 lecturas paralelas.

## Riesgos Clave y Mitigación

- **Precisión del detector de arquitectura**: evidencia explícita + fallback `unknown`.
- **Degradación de rendimiento en grafos grandes**: benchmarks continuos y límites por milestone.
- **Migraciones SQLite en datos reales**: backup automático + validación sobre DB con datos v1.
- **Desalineación contratos UI/BE**: contratos versionados + tests de integración.
- **Scope creep hacia v3**: gate explícito de no-goals y bloqueo de PR fuera de alcance.

## Rollout Intent

- Implementación incremental en PRs pequeños bajo budget de review (400 líneas objetivo).
- Orden sugerido: contratos+migración → insights/impacto → UX analítica/export → i18n foundation.
- Cada slice con pruebas (unit/integration) y validación de modo degradado.

## Aceptación de Implementación (Framing)

Se considera cumplido cuando:

1. Los flujos de arquitectura, impacto, ciclos/hotspots y export funcionan end-to-end.
2. Se respeta alcance v2 sin mezclar entregables v3.
3. Se ejecuta migración aditiva sin pérdida de datos v1.
4. Contratos v2 quedan versionados y validados en integración.
5. Foundation i18n queda operativa (`es.json` + `t('key')`) sin selector de idioma.
6. Se cumplen métricas aprobadas en Spec/Verify.

## Áreas Afectadas

- `engine/src/*` (graph insights, impact engine, architecture detection, commands, db/migrations)
- `engine/migrations/*` (nuevo `003_architecture_and_insights.sql`)
- `src/*` (vistas analíticas, filtros, export UX, wiring)
- `src/lib/types.ts` (contratos v2)
- `src/locales/es.json` y helper i18n
- `openspec/changes/v2-advanced-analysis/*`

## Rollback

Si un slice rompe contratos v1, estabilidad o performance objetivo, se revierte el slice completo, se restaura backup de DB si aplica, y se replanifica sin ampliar alcance.
