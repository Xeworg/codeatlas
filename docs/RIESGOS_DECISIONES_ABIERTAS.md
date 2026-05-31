# CodeAtlas — Riesgos y Decisiones Abiertas

**Versión:** pre-SDD v1
**Alcance:** registro vivo de riesgos técnicos, decisiones pendientes y sus dueños

---

## 1. Riesgos Técnicos

| ID | Riesgo | Probabilidad | Impacto | Mitigación | Dueño | Status | Target |
|---|---|---|---|---|---|---|---|
| R1 | Tree-sitter no maneja bien TSX/JSX complejos (generic types, decorators) | Media | Alto | Fixtures progresivos. Regex fallback para imports si falla parser. | Backend | 🔴 Open | Sprint 1 |
| R2 | Grafo ilegible con >1000 archivos | Alta | Alto | Agrupación por carpeta + lazy rendering + filtro por tipo. | Frontend | 🟡 Mitigated | Sprint 3 |
| R3 | IA devuelve respuestas genéricas o alucina | Media | Medio | Contexto acotado con código real. Métrica de fidelidad en v2. | Backend | 🟡 Mitigated | Sprint 4 |
| R4 | Performance de React Flow con >500 nodos visibles | Media | Alto | Virtualización, nivel de detalle por zoom, benchmark desde Sprint 3. | Frontend | 🔴 Open | Sprint 3 |
| R5 | API key management inseguro (keyring no disponible en headless) | Baja | Alto | Usar keyring nativo. Fallback: cifrado simétrico local con clave derivada de machine-id. | Backend | 🟡 Mitigated | Sprint 4 |
| R6 | Tauri v2 APIs inestables (breaking changes entre minor) | Media | Medio | Versión exacta en Cargo.toml. CI testea contra nightly cada 2 semanas. | Tech Lead | 🟡 Mitigated | Sprint 0 |
| R7 | Escaneo de 5000 archivos excede 10s por I/O lenta | Media | Alto | Profiling en Sprint 1. Optimizar con walkdir paralelo + buffer de lectura. | Backend | 🔴 Open | Sprint 1 |
| R8 | Contexto IA excede límite de tokens del modelo | Baja | Medio | Truncado agresivo + recorte automático si falla. | Backend | 🟢 Resolved | — |
| R9 | MiniMax vía Anthropic API no está disponible o cambia de endpoint | Media | Alto | Abstracción de proveedor. Fallback a Claude Haiku si MiniMax no disponible. | Backend | 🔴 Open | Sprint 0 |
| R10 | Colaboración v3 requiere backend server → scope creep masivo | Baja | Alto | Mantener colaboración local-first (snapshots exportables). Servidor solo si hay demanda validada. | Arquitectura | 🟡 Mitigated | v3 planning |

---

## 2. Decisiones de Arquitectura Pendientes

| ID | Decisión | Opciones | Recomendación | Dueño | Status | Target |
|---|---|---|---|---|---|---|
| D1 | ¿Auto-layout: Dagre o ELK? | Dagre (simple, rápido) / ELK (mejor calidad, más complejo) | Dagre para v1, ELK opcional en v2 | Frontend | 🔴 Open | Sprint 3 |
| D2 | ¿NodeType: heurística estática o ML? | Heurística basada en path/nombre/imports | Heurística simple en v1 | Backend | 🟢 Resolved | Sprint 2 |
| D3 | ¿Persistir layout de usuario o recalcular siempre? | Persistir en DB / Recalcular cada vez | Recalcular en v1, persistir en v2 | Frontend | 🟢 Resolved | Sprint 3 |
| D4 | ¿Tauri events (push) o polling desde frontend? | Events cada 500ms / Polling cada 500ms | Events desde BE para scan progress | Tech Lead | 🔴 Open | Sprint 1 |
| D5 | ¿Migraciones automáticas al iniciar o comando manual? | Auto en dev, preguntar en prod | Auto con backup automático | Backend | 🟢 Resolved | Sprint 0 |
| D6 | ¿Formato de snapshot: JSON plano o binary (bincode/msgpack)? | JSON (debuggable) / Binary (tamaño) | JSON para v3, binary si tamaño es problema | Arquitectura | 🟡 Deferred | v3 planning |
| D7 | ¿Un solo binario Tauri o engine como sidecar? | Engine integrado como crate / Engine separado como proceso | Crate integrado (más simple, menos IPC) | Tech Lead | 🟢 Resolved | Sprint 0 |
| D8 | ¿Tests E2E con WebDriver desde v1 o manual? | Manual v1 / WebDriver v2 | Manual con checklist en v1 | QA | 🟢 Resolved | Sprint 5 |

---

## 3. Decisiones de Producto Pendientes

| ID | Decisión | Opciones | Dueño | Status | Target |
|---|---|---|---|---|---|
| P1 | ¿Angular se incluye en MVP o no? | Best-effort solo imports / Excluido total | Producto | 🔴 Open | Sprint 1 |
| P2 | ¿Chat persiste entre sesiones de la app? | Solo en memoria v1 / Persistir en DB v1.1 | Producto | 🟢 Resolved | v1.1 |
| P3 | ¿Qué tamaño máximo de proyecto se garantiza? | 5,000 archivos / 10,000 archivos | Producto | 🟢 Resolved | — |
| P4 | ¿Licencia del producto? | MIT / Apache 2.0 / Proprietary | Producto | 🔴 Open | Pre-release |
| P5 | ¿Nombre final "CodeAtlas" o sujeto a cambio? | CodeAtlas / Alternativas | Producto | 🟢 Resolved | — |

---

## 4. Deuda Técnica Asumida (consciente)

| ID | Deuda | Razón | Plan de pago |
|---|---|---|---|
| DT1 | Sin virtualización de nodos en React Flow (v1) | Complejidad de implementación | v2 con vista de >1000 nodos |
| DT2 | Regex fallback para imports si Tree-sitter falla | Cobertura temporal | Remover cuando Tree-sitter sea estable |
| DT3 | Historial de chat solo en memoria | Evitar DB adicional en v1 | v1.1 con `chat_sessions` |
| DT4 | Sin i18n framework (solo español hardcodeado) | Velocidad de entrega | v2 con `react-i18next` |
| DT5 | Layout de grafo no persiste | Simplicidad | v2 con `graph_layout` en DB |

---

## 5. Supuestos Clave (sin verificar)

| ID | Supuesto | Validación | Dueño | Target |
|---|---|---|---|---|
| S1 | `tsconfig.json` paths son la única fuente de aliases | Probar con 5 proyectos reales | Backend | Sprint 2 |
| S2 | React Flow escala a 5000 nodos con técnicas básicas | Benchmark con 5000 nodos mock | Frontend | Sprint 3 |
| S3 | MiniMax M1 tiene calidad suficiente para explicar código | Evaluación manual con 10 archivos | Backend | Sprint 4 |
| S4 | SQLite con WAL mode no bloquea reads durante writes | Test de concurrencia | Backend | Sprint 2 |
| S5 | Tauri v2 es estable para producción en Q3 2026 | Revisar roadmap de Tauri | Tech Lead | Sprint 0 |

---

## 6. Proceso de gestión

- **Revisión semanal** de riesgos abiertos (viernes, 15 min).
- **Dueño** es responsable de avanzar o escalar.
- **Status:** 🔴 Open → 🟡 Mitigated/In Progress → 🟢 Resolved/Closed.
- Cambios de status se registran en este documento con fecha.

---

*Documento vivo. Actualizar semanalmente durante ejecución de sprints.*
