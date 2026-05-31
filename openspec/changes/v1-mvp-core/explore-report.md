# Explore Report — v1-mvp-core

**Fase:** Explore  
**Change ID:** v1-mvp-core  
**Fecha:** 2026-05-31  
**Estado:** completado ✅

---

## Resumen Ejecutivo

El MVP v1 de CodeAtlas está **correctamente acotado**. Sus fronteras son claras, los riesgos superables con las mitigaciones documentadas, y no hay conflictos entre los 12 documentos de referencia en `docs/`. Se recomienda pasar a fase **Proposal** sin cambios de alcance.

---

## Fronteras Congeladas de v1

### Lo que SÍ se construye (8 módulos)

| Módulo | Frontera exacta |
|---|---|
| **Scanner** | Walker Rust con exclusiones. Extensiones: `.ts`, `.tsx`, `.js`, `.jsx`, `.rs`, `.json`. |
| **Parser** | Tree-sitter: `typescript`, `javascript`, `rust`. Extrae imports, exports, functions, classes, structs, impl, interfaces, enums. |
| **Graph Engine** | Grafo **file-level** únicamente. Nodos=archivos. Aristas=imports. Sin nodos intra-archivo ni aristas de uso/llamadas. |
| **Graph Visualization** | React Flow: zoom, pan, auto-layout, search, highlight, colores por NodeType, agrupación colapsable. |
| **Project Explorer** | Sidebar read-only, árbol colapsable, sincronización con grafo. |
| **File Details** | Metadata del nodo: path, símbolos, dependencias, dependientes, NodeType. |
| **AI Context Engine** | Contexto acotado: archivo (8KB) + top-5 deps + top-3 dependents. Nunca proyecto completo. |
| **AI Assistant** | explain_node + chat. Historial en memoria (sin persistencia). Anthropic + MiniMax. |

### Lenguajes v1
**TypeScript, JavaScript, React, Node.js, Rust.**  
Fuera: Python, Java, Go, C#, Angular completo.

### SQLite v1
6 tablas: `projects`, `files`, `symbols`, `imports`, `graph_cache`, `ai_config`.  
**Sin** `chat_history`, `user_settings` (diferidos a v1.1).

---

## Supuestos Clave

| ID | Supuesto | Riesgo si falso |
|---|---|---|
| P2 | 8KB de archivo alcanza para explicaciones útiles de IA | Respuestas genéricas |
| T1 | Tree-sitter TS/JS/Rust ≥ 95% precisión en imports | Grafo incompleto |
| T2 | React Flow rinde con 5000 nodos agrupados | Migrar a Canvas/WebGL |
| P4 | MiniMax es compatible vía Anthropic API | Retrabajo en capa IA |

---

## Riesgos P0 (bloqueantes)

1. **Tree-sitter falla en edge cases reales.** → Fixtures progresivos + fallback regex.
2. **Grafo ilegible con 5000 archivos.** → Agrupación colapsable + lazy rendering.
3. **IA alucina o da respuestas genéricas.** → Contexto acotado + validación post-respuesta.
4. **Desalineación contratos UI-BE.** → Congelar tipos antes de cada sprint + contract tests.
5. **API key sin cifrar.** → Keyring nativo del SO.
6. **Tauri v2 APIs inestables.** → Versión fija en Cargo.toml.
7. **Escaneo > 10s.** → Profiling temprano + Rayon si necesario.

---

## No-Goals Explícitos (bloqueables en PR)

| Item | Fase |
|---|---|
| Detección automática de patrones | v2 |
| Grafo intra-archivo (clase/función) | v2 |
| Exportación Mermaid/PNG/SVG | v2 |
| Health score, ciclos, god classes | v2 |
| Chat history persistido | v1.1 |
| list_projects, delete_project | v1.1 |
| Multi-proyecto simultáneo | v3 |
| Colaboración, comentarios, snapshots | v3 |
| Edición de código | Nunca |

---

## Decisiones Técnicas Abiertas

| ID | Decisión | Recomendación |
|---|---|---|
| D1 | Auto-layout: Dagre vs ELK | Dagre (MVP). ELK en v2. |
| D2 | Compresión graph_json: zstd vs ninguna | Sin compresión en v1. |
| D3 | Rust parser: solo `use` o `use` + `mod` | Ambos en v1. |
| D4 | Paralelismo scanner: Rayon vs single-thread | Single-thread. Medir en Sprint 2. |

---

## Verificación de Consistencia

Los 12 documentos en `docs/` son **consistentes entre sí y con este explore report**. Sin conflictos detectados.

---

## Recomendación

**Pasar a fase Proposal** sobre el mismo change-id `v1-mvp-core`. Las fronteras están congeladas, los supuestos identificados, los riesgos mitigados, y los no-goals bloqueables.
