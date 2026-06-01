# CodeAtlas — Riesgos y Decisiones Abiertas

**Versión:** post-v2 (actualizado 2026-06-01)
**Alcance:** registro vivo — v1/v2 cerrados, v3 por iniciar

## 1. Riesgos Técnicos

| ID      | Riesgo                                                          | Probabilidad | Impacto | Mitigación                                                                                     | Status       | Target      |
| ------- | --------------------------------------------------------------- | ------------ | ------- | ---------------------------------------------------------------------------------------------- | ------------ | ----------- |
| R1      | Tree-sitter no maneja bien TSX/JSX complejos                    | Media        | Alto    | Fixtures progresivos. Regex fallback para imports si falla parser. Implementado en v1.         | ✅ Mitigated | —           |
| R2      | Grafo ilegible con >1000 archivos                               | Alta         | Alto    | Agrupación por carpeta + lazy rendering + filtro por tipo.                                     | ✅ Mitigated | v3          |
| R3      | IA devuelve respuestas genéricas o alucina                      | Media        | Medio   | Contexto acotado con código real. Métrica de fidelidad en v2.                                  | ✅ Mitigated | —           |
| R4      | Performance de React Flow con >500 nodos visibles               | Media        | Alto    | Virtualización, nivel de detalle por zoom, benchmark desde Sprint 3.                           | ✅ Mitigated | v3          |
| R5      | API key management inseguro (keyring no disponible en headless) | Baja         | Alto    | Usar keyring nativo. Fallback: cifrado simétrico local con clave derivada de machine-id.       | ✅ Mitigated | —           |
| R6      | Tauri v2 APIs inestables (breaking changes entre minor)         | Media        | Medio   | Versión exacta en Cargo.toml. CI testea contra nightly cada 2 semanas.                         | ✅ Mitigated | —           |
| R7      | Escaneo de 5000 archivos excede 10s por I/O lenta               | Media        | Alto    | Profiling en Sprint 1. Optimizar con walkdir paralelo + buffer de lectura. Implementado en v1. | ✅ Mitigated | —           |
| R8      | Contexto IA excede límite de tokens del modelo                  | Baja         | Medio   | Truncado agresivo + recorte automático si falla.                                               | ✅ Resolved  | —           |
| R9      | MiniMax vía Anthropic API no disponible o cambia de endpoint    | Media        | Alto    | Abstracción de proveedor. Fallback a Claude Haiku si MiniMax no disponible.                    | ✅ Mitigated | —           |
| R10     | Colaboración v3 requiere backend server → scope creep masivo    | Baja         | Alto    | Mantener colaboración local-first (snapshots exportables).                                     | ✅ Mitigated | v3 planning |
| **R11** | **v3: Workspace multi-proyecto con isolation de DB**            | Media        | Alto    | Schema `workspaces` + `workspace_projects`. Migración aditiva.                                 | 🔴 Open      | v3          |
| **R12** | **v3: Snapshot round-trip integrity (hash validation)**         | Media        | Alto    | Formato `.codeatlas-snapshot` con hash SHA-256. Tests de integración.                          | 🔴 Open      | v3          |
| **R13** | **v3: App-level wiring incompleto (T5.6 diferido de v2)**       | Media        | Medio   | Componentes wire-ready pero no integrados en `App.tsx`.                                        | 🔴 Open      | Early v3    |
| **R14** | **v3: NFR benchmarks sin mediciones reales (scaffold only)**    | Baja         | Medio   | Crear fixture de 1000+ archivos. Correr benchmarks contra thresholds antes de Beta.            | 🔴 Open      | Early v3    |

## 2. Decisiones de Arquitectura Pendientes

| ID      | Decisión                                                    | Opciones                                                   | Recomendación                                                | Status      | Target  |
| ------- | ----------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------ | ----------- | ------- |
| D1      | ¿Auto-layout: Dagre o ELK?                                  | Dagre (simple, rápido) / ELK (mejor calidad, más complejo) | Dagre para v1-v2, ELK opcional en v3                         | ✅ Resolved | —       |
| D2      | ¿NodeType: heurística estática o ML?                        | Heurística basada en path/nombre/imports                   | Heurística simple en v1-v2                                   | ✅ Resolved | —       |
| D3      | ¿Persistir layout de usuario o recalcular siempre?          | Persistir en DB / Recalcular cada vez                      | Persistir en v3                                              | 🔴 Open     | v3      |
| D4      | ¿Tauri events (push) o polling desde frontend?              | Events cada 500ms / Polling cada 500ms                     | Events desde BE para scan progress. Implementado en v1.      | ✅ Resolved | —       |
| D5      | ¿Migraciones automáticas al iniciar o comando manual?       | Auto en dev, preguntar en prod                             | Auto con backup automático. Implementado.                    | ✅ Resolved | —       |
| D6      | ¿Formato de snapshot: JSON plano o binary?                  | JSON (debuggable) / Binary (tamaño)                        | JSON para v3                                                 | 🔴 Open     | v3      |
| D7      | ¿Un solo binario Tauri o engine como sidecar?               | Engine integrado como crate / Engine separado como proceso | Crate integrado (más simple, menos IPC)                      | ✅ Resolved | —       |
| D8      | ¿Tests E2E con WebDriver desde v1 o manual?                 | Manual v1 / WebDriver v2                                   | Manual con checklist en v1-v2. WebDriver diferido a v3 E2E.  | 🟡 Deferred | v3      |
| **D9**  | **v3: ¿Workspace comparte DB o DB separada por proyecto?**  | Shared SQLite / DB por proyecto                            | SQLite compartido con `workspaces` table. Migración aditiva. | 🔴 Open     | v3 spec |
| **D10** | **v3: ¿Snapshots incluyen grafo completo o solo metadata?** | Grafo + symbols / Solo metadata                            | Grafo serializado + hash de integridad.                      | 🔴 Open     | v3 spec |

## 3. Decisiones de Producto Pendientes

| ID     | Decisión                                                                        | Opciones                                      | Status                                                      | Target      |
| ------ | ------------------------------------------------------------------------------- | --------------------------------------------- | ----------------------------------------------------------- | ----------- | --- |
| P1     | ¿Angular se incluye en MVP o no?                                                | Best-effort solo imports / Excluido total     | ✅ Mitigated (excluido total en v1-v2)                      | —           |
| P2     | ¿Chat persiste entre sesiones de la app?                                        | Solo en memoria v1 / Persistir en DB v1.1     | ✅ Resolved (en memoria v1-v2; DB en v3)                    | —           |
| P3     | ¿Qué tamaño máximo de proyecto se garantiza?                                    | 5,000 / 10,000 archivos                       | ✅ Resolved (5,000)                                         | —           |
| P4     | ¿Licencia del producto?                                                         | MIT / Apache 2.0 / Proprietary                | 🔴 Open                                                     | Pre-release |
| P5     | ¿Nombre final "CodeAtlas" o sujeto a cambio?                                    | CodeAtlas / Alternativas                      | ✅ Resolved (CodeAtlas)                                     | —           |
| **P6** | **v3: ¿Workspace es visible para el usuario final o solo modelo interno?**      | Visible como feature / Solo isolation técnica | 🔴 Open                                                     | v3 spec     |
| **P7** | **v3: ¿Snapshots colaborativos requieren servidor o son export/import manual?** | Server-based / Local-first export-import      | Local-first con export-import; servidor si demanda validada | 🟢 Resolved | v3  |

## 4. Deuda Técnica Asumida (consciente)

| ID  | Deuda                                            | Razón                                  | Plan de pago                               |
| --- | ------------------------------------------------ | -------------------------------------- | ------------------------------------------ |
| DT1 | Sin virtualización de nodos en React Flow        | Complejidad de implementación          | v3 con vista de >1000 nodos                |
| DT2 | Regex fallback para imports si Tree-sitter falla | Cobertura temporal                     | v3: remover cuando Tree-sitter sea estable |
| DT3 | Historial de chat solo en memoria                | Evitar DB adicional en v1-v2           | v3 con `chat_sessions`                     |
| DT4 | Sin i18n framework (es.json + t())               | Velocidad de entrega v1-v2             | v2 con `es.json` + `t()`; en.json en v3+   |
| DT5 | Layout de grafo no persiste                      | Simplicidad v1-v2                      | v3 con `graph_layout` en DB                |
| DT6 | NFR benchmarks scaffold (sin fixture real)       | Prioridad de features sobre mediciones | Early v3 hardening                         |
| DT7 | App-level wiring T5.6 diferido de v2             | Scope management                       | Early v3                                   |

## 5. Supuestos Clave (v1/v2 verificados)

| ID  | Supuesto                                                 | Validación                        | Estado                           |
| --- | -------------------------------------------------------- | --------------------------------- | -------------------------------- |
| S1  | `tsconfig.json` paths son la única fuente de aliases     | Probar con 5 proyectos reales     | ✅ Verificado en v1              |
| S2  | React Flow escala a 5000 nodos con técnicas básicas      | Benchmark con 5000 nodos mock     | ⚠️ Necesita virtualización en v3 |
| S3  | MiniMax M1 tiene calidad suficiente para explicar código | Evaluación manual con 10 archivos | ✅ Mitigated en v1               |
| S4  | SQLite con WAL mode no bloquea reads durante writes      | Test de concurrencia              | ✅ Verificado en v2              |
| S5  | Tauri v2 es estable para producción en Q3 2026           | Revisar roadmap de Tauri          | ✅ Mitigated                     |

---

## 6. Proceso de gestión

- **Revisión** de riesgos abiertos antes de cada SDD phase gate.
- **Status:** 🔴 Open → 🟡 Mitigated/In Progress → 🟢 Resolved/Closed.
- Cambios de status se registran en este documento.

_Documento vivo. Actualizado post-v2 archive (2026-06-01)._
