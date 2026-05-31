# Proposal — v1-mvp-core

## Resumen
Este cambio formaliza el **MVP v1 de CodeAtlas** para entregar comprensión rápida de codebases mediante escaneo estático, grafo de dependencias file-level, exploración visual interactiva y explicación contextual con IA.

## Intento / Objetivo v1
Reducir el tiempo de entendimiento inicial de un proyecto nuevo con una experiencia end-to-end:
1. Abrir carpeta de proyecto.
2. Escanear y parsear código (JS/TS/React/Node/Rust).
3. Construir y visualizar grafo file-level.
4. Inspeccionar nodos (detalles, dependencias, dependientes).
5. Obtener explicaciones IA contextuales (archivo + vecindad inmediata).

## Alcance (In Scope)
### Backend
- Scanner estático seguro con exclusiones estándar.
- Parser Tree-sitter para JS/TS/Rust.
- Motor de grafo dirigido a nivel archivo (nodos=archivos, aristas=imports).
- Persistencia SQLite mínima: `projects`, `files`, `symbols`, `imports`, `graph_cache`, `ai_config`.
- Comandos Tauri v1: scan/status/graph/node-details/search + explain/chat.

### Frontend
- Layout v1: Explorer (izq), Grafo (centro), IA (der), detalles (inferior), top/status bar.
- Interacción de grafo: zoom, pan, búsqueda, auto-layout, highlight.
- Explorer read-only sincronizado con selección de nodo.
- Panel de detalles de archivo.
- Asistente IA contextual (explain_node + chat) con historial en memoria.

### IA
- Proveedor primario: Anthropic.
- Primer modelo operativo: MiniMax (detrás de abstracción).
- Contexto acotado: archivo (hasta ~8KB) + top-5 dependencias + top-3 dependientes.

## Límites de Alcance (Out of Scope / Non-Goals)
- Detección automática de patrones arquitectónicos (v2).
- Grafo intra-archivo (clase/función) y aristas de uso/llamadas (v2).
- Export Mermaid/PNG/SVG (v2).
- Health score, ciclos y hotspots avanzados (v2).
- Persistencia de chat, `list_projects`, `delete_project` (v1.1).
- Multi-proyecto y colaboración (v3).
- Edición de código (fuera de producto).

## Métricas de Éxito v1
- Escaneo inicial (≤5000 archivos): **<10s**.
- Primer diagrama visible: **<30s**.
- Interacción de grafo: **<100ms** latencia percibida.
- Respuesta IA contextual: **<5s**.
- Seguridad: no ejecución de código analizado y API key protegida en keyring.

## Riesgos Clave y Mitigación
- Precisión parser en edge-cases: fixtures progresivos + fallback controlado.
- Legibilidad/performance de grafos grandes: agrupación colapsable + lazy rendering.
- Desalineación UI/BE: contratos congelados por sprint + contract tests.
- Respuestas IA genéricas/alucinación: contexto acotado + validación de referencias.

## Rollout Intent
- Implementación incremental por slices UI/BE coordinados bajo budget de revisión (400 líneas objetivo).
- Entrega de demo funcional v1 al cierre de milestones:
  1) scan+layout,
  2) grafo+explorer,
  3) detalles+IA.
- Sin mezclar features v2/v3 antes de cumplir criterios de salida v1.

## Aceptación de Implementación (Framing)
Se considera cumplido cuando:
1. El flujo abrir→escanear→visualizar→inspeccionar→preguntar funciona de punta a punta.
2. Se respetan los límites de alcance v1 sin incluir no-goals.
3. Se cumplen objetivos de performance y seguridad definidos.
4. Contratos TS↔Rust permanecen consistentes y verificados por tests.
5. CI y estándares de calidad del proyecto pasan sin excepciones.

## Áreas Afectadas
- `engine/` (scanner, parser, graph, ai, db, tauri commands)
- `src/` (layout, explorer, graph, panel detalles, assistant)
- `src/lib/types.ts` y wrappers Tauri
- `openspec/changes/v1-mvp-core/*` y docs de soporte

## Rollback
Si alguna entrega parcial rompe objetivos v1 (performance, seguridad, contrato), se revierte el slice completo y se replanifica en el siguiente PR sin ampliar alcance.
