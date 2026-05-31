# CodeAtlas — Plan Maestro UI + Backend (v1 a v3)

Plan integral para ejecutar producto por versiones, coordinando frontend y backend sin romper alcance.

---

## 1) Enfoque general

- **v1:** MVP funcional de comprensión de proyecto (rápido, claro, estable).
- **v2:** análisis técnico avanzado (impacto, arquitectura detectada, exportes).
- **v3:** plataforma colaborativa multi-proyecto con capa ejecutiva.

Regla: no se adelantan features de una versión si comprometen el cierre de la anterior.

---

## 2) v1 — MVP

## Objetivo
Abrir proyecto → escanear → ver grafo → inspeccionar nodo → obtener explicación IA contextual.

## UI (v1)
- Shell y layout 3 columnas + panel inferior
- Explorer read-only
- Grafo file-level (zoom/pan/search/highlight)
- Panel de detalles
- Sidebar IA con chat contextual
- Tipos de diagrama v1: **Mapa de Arquitectura** y **Vista de Dependencias**

## Backend (v1)
- Scanner estático seguro
- Parser Tree-sitter JS/TS/Rust
- Grafo file-level
- SQLite mínima
- Endpoints Tauri para scan/graph/details/search
- Context engine + explain/chat IA
- Proveedor IA primario: **Anthropic**; primer modelo operativo: **MiniMax**

## Integraciones UI-BE (v1)
1. Contratos de tipos (`FileInfo`, `GraphData`, `NodeDetails`, `ChatResponse`)
2. Comandos Tauri estables
3. Estados compartidos (`loading/ready/empty/error`)

## Hitos v1
- **H1:** Layout y scanner base integrados
- **H2:** Grafo real integrado con explorer
- **H3:** Detalles + IA contextual en demo completa

## Criterio de salida v1
- Demo end-to-end estable en proyecto real objetivo
- Performance dentro de umbrales definidos
- Sin features de v2/v3 mezcladas

---

## 3) v2 — Profundización técnica

## Objetivo
Mejorar capacidad de análisis de impacto y lectura arquitectónica con evidencia visual.

## UI (v2)
- Modos de vista: Arquitectura / Dependencias / Flujo
- Filtros persistentes y agrupaciones
- Vista de ciclos y hotspots
- Tarjeta arquitectura detectada con confianza
- Export básico de vistas
- Tipo de diagrama adicional: **Flujo de aplicación (beta)**

## Backend (v2)
- Detector de arquitectura (MVC/Layered/Clean/Hexagonal) con score
- Aristas avanzadas (usage/calls) progresivas
- Motor de impacto de cambios
- Cálculos de ciclos y métricas básicas de acoplamiento
- Endpoint de exportes

## Integraciones UI-BE (v2)
1. Contrato de `ArchitectureDetectionResult`
2. Contrato de `ImpactAnalysisResult`
3. Contrato de `GraphInsights` (ciclos/hotspots)

## Hitos v2
- **H1:** insights arquitectónicos visibles
- **H2:** impacto de cambios usable
- **H3:** export funcional

## Criterio de salida v2
- Usuario responde “qué afecta este cambio” en minutos
- Arquitectura detectada con evidencia trazable
- Export confiable para compartir

---

## 4) v3 — Plataforma colaborativa

## Objetivo
Pasar de herramienta individual a espacio de trabajo arquitectónico para equipos.

## UI (v3)
- Multi-proyecto
- Snapshots compartibles
- Comentarios/anotaciones
- Dashboard ejecutivo + vistas C4 asistidas
- Tipos de diagrama ejecutivos: **C4 asistido (Nivel 1/2)** y comparativas entre snapshots

## Backend (v3)
- Modelo de datos para colaboración
- Persistencia de snapshots/comentarios
- Historial temporal de health score
- Servicios para agregación ejecutiva y comparativas

## Integraciones UI-BE (v3)
1. Contrato de colaboración (`Comment`, `Snapshot`, `SharedView`)
2. Contrato de salud (`HealthScoreTimeline`)
3. Contrato de resumen ejecutivo (`ExecutiveArchitectureSummary`)

## Hitos v3
- **H1:** multi-proyecto estable
- **H2:** colaboración básica operativa
- **H3:** panel ejecutivo con evolución temporal

## Criterio de salida v3
- Equipos colaboran sobre arquitectura en una misma plataforma
- Vistas compartidas reproducibles
- Métricas históricas comparables

---

## 5) Trabajo paralelo UI + Backend por versión

## v1
- **Paralelo 1:** UI shell/explorer ↔ BE scanner/parser
- **Paralelo 2:** UI grafo ↔ BE graph engine/endpoints
- **Paralelo 3:** UI IA panel ↔ BE context+chat/explain

## v2
- **Paralelo 1:** UI filtros/modos ↔ BE insights/ciclos
- **Paralelo 2:** UI impacto visual ↔ BE motor impacto
- **Paralelo 3:** UI export ↔ BE serialización/export

## v3
- **Paralelo 1:** UI colaboración ↔ BE modelo snapshots/comentarios
- **Paralelo 2:** UI dashboard ejecutivo ↔ BE agregación temporal
- **Paralelo 3:** UI multi-proyecto ↔ BE servicios de switching/contexto

---

## 6) Dependencias maestras

1. Contrato de tipos versionado antes de cada release.
2. Tests de integración UI-BE en cada hito.
3. Performance budget validado al cierre de cada versión.
4. Seguridad de análisis estático como restricción permanente.

---

## 7) Riesgos transversales y mitigación

- **Riesgo:** desalineación de contratos UI-BE.  
  **Mitigación:** congelar contratos por milestone y versionarlos.

- **Riesgo:** scope creep de features avanzadas.  
  **Mitigación:** tablero por versión + gate de aceptación.

- **Riesgo:** degradación de performance en grafos grandes.  
  **Mitigación:** profiling continuo + límites de rendering.

- **Riesgo:** IA poco confiable.  
  **Mitigación:** contexto acotado, evidencia visible, manejo de errores robusto.

---

## 8) Cadencia recomendada

- Planning semanal UI-BE conjunto
- Daily técnico corto
- Demo quincenal por hitos
- Retro al cierre de cada versión

---

## 9) Entregables documentales por versión

## v1
- Spec UI v1
- Tablero Sprint UI v1
- Tablero Sprint Backend v1
- Contratos Tauri/Types v1

## v2
- Spec UI v2
- Spec Backend insights v2
- Contratos de arquitectura/impacto/export v2

## v3
- Spec colaboración y multi-proyecto
- Contratos de snapshots/comentarios/health timeline
- Guía ejecutiva de arquitectura

---

## 10) Definición de éxito global (v1→v3)

- **v1:** comprensión rápida individual del codebase.
- **v2:** análisis de impacto y arquitectura con evidencia.
- **v3:** colaboración y gobernanza arquitectónica de equipo.
