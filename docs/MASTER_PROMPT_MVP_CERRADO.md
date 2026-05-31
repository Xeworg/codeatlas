# CODEATLAS MVP — MASTER IMPLEMENTATION PROMPT (ALCANCE CERRADO)

**Versión:** MVP v1.0  
**Idioma:** español (ES-ar)  
**Propósito:** guía ejecutiva para implementar el MVP de CodeAtlas sin scope creep.  
**Regla de oro:** si una feature no está en este documento, **no se construye ahora**.

---

## PRODUCT VISION

CodeAtlas es una aplicación de escritorio con IA que ayuda a desarrolladores a entender proyectos de software mediante diagramas de arquitectura visuales interactivos, grafos de dependencia y explicaciones contextuales en tiempo real.

**No es un IDE. No es un editor de código.**  
Es una plataforma de comprensión de proyectos y visualización arquitectónica.

---

## MVP OBJECTIVE

Cuando un usuario abre una carpeta de proyecto:

1. La app escanea el codebase de forma estática.
2. Construye un grafo de dependencias a nivel de archivo.
3. Renderiza un mapa visual interactivo.
4. Permite explorar relaciones entre archivos.
5. Provee explicaciones IA contextuales por archivo.
6. Ofrece un chat contextual que usa el código real como evidencia.

---

## SUPPORTED LANGUAGES (MVP)

Solo:

- TypeScript
- JavaScript
- React (JSX/TSX)
- Node.js
- Rust

**Fuera de alcance:** Python, Java, Go, C#, Angular completo (best effort solo imports).

---

## DESKTOP TECHNOLOGY STACK

| Capa | Tecnología |
|---|---|
| Desktop Shell | Tauri v2 |
| Frontend | React 18 + TypeScript + Vite |
| UI / Estilos | Tailwind CSS |
| Diagram Engine | React Flow (xyflow) |
| State Management | Zustand |
| Database | SQLite (rusqlite) |
| Backend Engine | Rust |
| Code Parsing | Tree-sitter (binding Rust) |
| AI Provider | Anthropic (primario) vía capa abstracta; modelo inicial: MiniMax |

**Exclusiones UX para MVP:** no se requieren shadcn/ui, React Hook Form ni Zod. Tailwind puro + componentes simples alcanzan.

**Estrategia IA MVP:**
- Proveedor primario: **Anthropic**.
- Primer modelo operativo: **MiniMax** (siempre detrás de la capa abstracta de proveedor/modelo).
- Sin multi-proveedor simultáneo en v1.

---

## CORE MODULES (MVP)

### 1. PROJECT SCANNER

**Responsabilidades:**
- Abrir carpeta de proyecto vía diálogo nativo Tauri.
- Recorrer filesystem con walker Rust.
- Filtrar extensiones: `.ts`, `.tsx`, `.js`, `.jsx`, `.json`.
- Ignorar: `node_modules`, `dist`, `build`, `.git`, `.next`, `coverage`.
- Generar índice de archivos con paths relativos y hashes de contenido.

**Output:** `ScanResult` con lista de `FileInfo`.

### 2. CODE PARSER

Usando Tree-sitter (gramáticas `tree-sitter-typescript`, `tree-sitter-javascript` y `tree-sitter-rust`).

**Extraer:**
- `imports` (source, target module/path, imported symbols)
- `exports` (named, default)
- `functions`
- `classes` (JS/TS) / `structs` + `impl` (Rust, cuando aplique)
- `interfaces` (TS)
- `type aliases`
- `enums`

**Output:** `CodeMetadata` asociado a cada archivo.

### 3. DEPENDENCY GRAPH ENGINE

Construir grafo dirigido **a nivel de archivo**.

- **Nodo:** archivo (con clasificación por tipo: component, route, service, repository, model, util, config, test, external, unknown).
- **Arista:** relación de import entre archivos.

**Incluye:**
- Resolución de paths relativos.
- Resolución de aliases de TypeScript (`tsconfig.json` → `paths`).
- Módulos externos (`node_modules`) como nodos hoja especiales.
- Serialización a JSON para el frontend.

**Excluido de MVP:**
- Nodos intra-archivo (clase, función).
- Aristas de uso/llamadas entre archivos.
- Detección de patrones arquitectónicos automática.

**Output:** `GraphData` (nodes + edges).

### 4. GRAPH VISUALIZATION ENGINE

Usar React Flow.

**Features MVP:**
- Zoom y pan.
- Auto-layout (Dagre o ELK).
- Búsqueda de nodos con resaltado.
- Selección de nodo (click).
- Highlight de dependencias al hover.
- Colores por tipo de nodo (component, service, route, etc.).
- Agrupación colapsable por carpeta.
- Minimapa.

**Fuera de alcance:**
- Nodos intra-archivo (clase, función) — v2.
- Aristas de uso/llamadas — v2.

### 5. PROJECT EXPLORER

Sidebar izquierdo.

**Features:**
- Árbol de carpetas/archivos (solo lectura).
- Colapsar/expandir carpetas.
- Búsqueda textual.
- Sincronización con selección en grafo.

**No incluye:** edición de código, creación de archivos.

### 6. FILE DETAILS PANEL

Al seleccionar un nodo (archivo):

**Mostrar:**
- Nombre y path relativo.
- Extensión y cantidad de líneas.
- Símbolos extraídos (clases, funciones, interfaces).
- Lista de dependencias (qué archivos importa).
- Lista de dependientes (qué archivos lo importan).
- Tipo de nodo clasificado.

### 7. AI CONTEXT ENGINE

Preparar contexto para IA.

**Reglas duras:**
- **Nunca** enviar el proyecto completo.
- Enviar solo: archivo seleccionado + dependencias inmediatas + metadatos del grafo.
- Contexto compacto, cabe en ventana de tokens estándar.

### 8. AI ASSISTANT PANEL

Dos modos:

**A. Explicación por nodo:**
- Al hacer clic en un nodo → "Explicar este archivo".
- Respuesta: resumen + detalles en Markdown + rol en la arquitectura.

**B. Chat contextual:**
- Preguntas libres con contexto del proyecto.
- Historial de conversación en memoria (no persistido en esta fase).
- Referencias a nodos en la respuesta.

**Preguntas soportadas:**
- "¿Qué hace este archivo?"
- "¿Qué dependencias tiene?"
- "Explicame este flujo."
- "¿Cómo se relaciona con el resto del proyecto?"

**Restricción de seguridad:** solo se envía a la IA el fragmento de código consultado + dependencias inmediatas. El código fuente **nunca sale completo de la máquina**.

---

## TIPOS DE DIAGRAMA POR VERSIÓN

### v1 (MVP)
- **Mapa de Arquitectura (principal):** grafo de dependencias file-level.
- **Vista de Dependencias:** mismo grafo con foco en inbound/outbound por nodo.

### v2
- **Flujo de aplicación (beta):** recorridos entre rutas, servicios y repositorios cuando haya señal suficiente.
- **Vista de ciclos y hotspots:** superposición de insights sobre el grafo.

### v3
- **C4 asistido (Nivel 1/2):** vista ejecutiva para comunicación de arquitectura.
- **Snapshots comparativos:** comparación visual entre estados/versiones del proyecto.

## USER INTERFACE LAYOUT

```
┌─────────────────────────────────────────────────────────┐
│ Top Bar: Project Name | Search | Scan Status             │
├──────────┬────────────────────────┬──────────────────────┤
│          │                        │                      │
│ Project  │   Interactive Graph    │   AI Assistant       │
│ Explorer │   (React Flow)         │   (Chat / Explain)   │
│          │                        │                      │
│          │                        │                      │
├──────────┴────────────────────────┴──────────────────────┤
│ Status Bar: File Count | Dep Count | Scan Duration       │
└─────────────────────────────────────────────────────────┘
```

**Panel inferior (detalles):** se muestra al seleccionar nodo, reemplaza o comprime el grafo.

**Diseño:** moderno, tema oscuro, profesional, orientado a desarrollador.  
**Referencias visuales:** Linear, Raycast, VS Code — sin clutter.

---

## LOCAL STORAGE (SQLite)

Tablas MVP:

| Tabla | Propósito |
|---|---|
| `projects` | Proyecto escaneado (id, nombre, root_path, estado, stats). |
| `files` | Archivos encontrados (id, project_id, path, nombre, extensión, líneas, hash). |
| `symbols` | Símbolos por archivo (id, file_id, nombre, kind, línea, exportado). |
| `imports` | Relaciones de import (source_file_id, target_file_id, target_module). |
| `graph_cache` | Grafo serializado (project_id, graph_json). |
| `ai_config` | Configuración de IA (singleton, api_key encriptada con keyring del SO). |

**Diferido a v1.1:**
- `chat_history` (MVP mantiene historial en memoria del frontend).
- `user_settings` avanzadas (MVP usa configuración mínima).

---

## PERFORMANCE REQUIREMENTS

| Métrica | Objetivo |
|---|---|
| Escaneo inicial (≤5000 archivos) | < 10 segundos |
| Latencia de interacción en grafo | < 100ms |
| Memoria (proyecto mediano) | < 500 MB |
| Primer diagrama visible | < 30 segundos |
| Respuesta de IA | < 5 segundos |

---

## SECURITY REQUIREMENTS

- **Nunca ejecutar código del proyecto.**
- **Nunca correr scripts.**
- **Nunca evaluar código fuente.**
- Solo análisis estático de archivos.
- Tratar todo proyecto como entrada no confiable.
- API key de IA almacenada con keyring nativo del SO (nunca en texto plano).
- Solo se envía a la IA el fragmento consultado + dependencias inmediatas.

---

## MVP EXCLUSIONS (NO SE CONSTRUYE AHORA)

| Feature | Fase |
|---|---|
| Detección automática de patrones (MVC, Clean, Hexagonal) con score | v2 |
| Grafo semántico intra-archivo (clases, funciones como nodos) | v2 |
| Aristas de uso/llamadas entre archivos | v2 |
| Generación de documentación (README, C4) | v3 |
| ERD / diagramas de base de datos | v2 |
| Docker / Kubernetes visualization | v3 |
| Health score (deuda técnica, god classes, dep. circulares) | v2 |
| Exportación a Mermaid / PNG / SVG | v2 |
| Edición de código / refactoring | Fuera de scope |
| Git integration | v3 |
| Multi-proyecto simultáneo | v3 |
| Sincronización cloud / multi-usuario | Fuera de scope |
| Modelos locales (LLM on-device) | v2 |
| Soporte lenguajes adicionales (Python, Go, Rust, Java) | v2+ |
| Angular completo (best effort solo imports en MVP) | v2 |

---

## ACCEPTANCE CRITERIA (MVP)

1. **AC1:** Proyecto TS/JS ≤ 5000 archivos escanea y muestra grafo en < 30s.
2. **AC2:** Grafo navegable (zoom, pan, clic) sin lag perceptible (< 100ms).
3. **AC3:** Seleccionar nodo → explicación IA contextual en < 5s.
4. **AC4:** Chat contextual responde con referencias reales al código del proyecto.
5. **AC5:** App funciona en Linux, macOS y Windows sin diferencias de comportamiento.
6. **AC6:** Código del proyecto nunca se ejecuta ni se envía completo a ningún servidor.
7. **AC7:** API key se almacena cifrada con keyring del sistema operativo.

---

## DEFINITION OF DONE (por feature)

- [ ] Código en `main` con PR revisado.
- [ ] Tipos sincronizados entre TypeScript y Rust (contrato canónico).
- [ ] Tests unitarios pasando (`cargo test` + `vitest`).
- [ ] Test de integración con fixture cuando aplica.
- [ ] CI verde sin regresiones.
- [ ] Performance dentro de objetivos declarados (medido).
- [ ] Documentado en contrato de API (`tauri-api.ts` actualizado).

---

## GUARDRAILS ANTI-SCOPE-CREEP

1. **Si una feature no está en este documento, no se construye.**
2. **Una carpeta = un proyecto.** Sin multi-proyecto, sin workspaces complejos.
3. **Escaneo completo, sin incremental.** Sin watchers de archivos.
4. **Un solo proveedor IA en v1.** Anthropic como primario con MiniMax como primer modelo operativo (sin multi-proveedor simultáneo, sin fallback complejo, sin modelos locales).
5. **Sin exportación.** Nada de Mermaid, PNG, SVG.
6. **Sin health score ni detección de patrones.** La IA puede mencionar patrones en lenguaje natural, pero no hay clasificador automático.
7. **Persistencia mínima.** `chat_history` y features administrativas (`list_projects`, `delete_project`) pasan a v1.1 si comprometen la fecha del MVP.

### Proceso de decisión para nuevas ideas:

1. ¿Está en este documento? → **Sí:** se planifica. **No:** backlog v2/v3.
2. ¿Es necesaria para que el MVP funcione? → **Sí:** discusión de trade-off con equipo. **No:** backlog.
3. ¿Agrega más de 3 días de trabajo? → **Sí:** siguiente fase, sin excepción.

---

*Documento generado para CodeAtlas MVP v1.0. Alcance cerrado. Última actualización: 2026-05-31.*
