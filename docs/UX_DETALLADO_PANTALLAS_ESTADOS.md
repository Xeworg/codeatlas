# CodeAtlas — Diseño UX Detallado por Pantalla y Estado

**Versión:** pre-SDD v1
**Alcance:** especificación visual completa de cada pantalla, estado y transición

---

## 1. Estados globales de la aplicación

| Estado | Gatillo | Comportamiento visual |
|---|---|---|
| `init` | App recién abierta, sin proyecto | Welcome screen con CTA "Abrir proyecto". Top bar muestra "CodeAtlas". |
| `scanning` | Usuario selecciona carpeta | Explorer muestra skeleton. Centro: spinner + progreso. IA panel: deshabilitado. |
| `building_graph` | Scan completado | Explorer poblado. Centro: skeleton de grafo → transición a nodos. |
| `ready` | Grafo listo | Todo funcional. |
| `error` | Fallo en scan/parse | Explorer vacío. Centro: mensaje de error + botón "Reintentar". |
| `no_ai_key` | Sin API key configurada | IA panel muestra placeholder: "Configurá tu API key para activar la IA". |

---

## 2. Pantalla 1 — Welcome / Sin proyecto

### 2.1 Layout
```
┌──────────────────────────────────────────┐
│ Top Bar: CodeAtlas           [Config] [⚙]│
├──────────────────────────────────────────┤
│                                          │
│         🗺️  CodeAtlas                    │
│                                          │
│    Entendé arquitectura de software      │
│    con visualización interactiva e IA.   │
│                                          │
│    ┌──────────────────────────┐          │
│    │  📂  Abrir proyecto       │          │
│    └──────────────────────────┘          │
│                                          │
│    Proyectos recientes:                  │
│    ┌─ my-app        (hace 2h)           │
│    ├─ api-service   (ayer)              │
│    └─ dashboard     (3 días)            │
│                                          │
└──────────────────────────────────────────┘
```

### 2.2 Elementos
- **Logo + nombre** centrados.
- **Subtítulo**: una línea de propuesta de valor.
- **Botón primario** "Abrir proyecto" → diálogo nativo de carpeta.
- **Lista de recientes** (últimos 5, opcional si no hay DB aún: leer de `projects` table).
- **Acceso a Settings** (tuerca arriba derecha).

### 2.3 Estados de este componente
| Estado | Visual |
|---|---|
| Sin recientes | Sin lista, solo botón "Abrir proyecto" |
| Con recientes | Lista con nombre + timestamp relativo |
| Error al cargar recientes | Toast: "No se pudieron cargar proyectos recientes" |

---

## 3. Pantalla 2 — Proyecto cargado (Ready)

### 3.1 Layout completo
```
┌──────────────────────────────────────────────────────────────┐
│ 🗺️ CodeAtlas │ my-app  │ 🔍 Buscar... │ ✅ Listo │ ⚙ │ ⬜ ✕  │
├────────────┬────────────────────────────┬────────────────────┤
│ EXPLORER   │                            │  AI ASSISTANT      │
│            │                            │                    │
│ 📁 src/    │     [grafo interactivo]    │  💬 Chat           │
│  📁 comp/  │                            │  ┌──────────────┐  │
│    📄 A.tsx│     ○ ──→ ○                │  │¿Qué hace     │  │
│    📄 B.tsx│     │     │                │  │AuthService?  │  │
│  📁 svc/   │     ○ ←── ○                │  ├──────────────┤  │
│    📄 C.ts  │                            │  │Explicá este  │  │
│            │                            │  │flujo         │  │
│            │                            │  └──────────────┘  │
│            │                            │                    │
│            │                            │  ─────────────────  │
│            │                            │  [Escribí tu       │
│            │                            │   pregunta...]  📤  │
├────────────┴────────────────────────────┴────────────────────┤
│ 📄 AuthService.ts │ ▸ UserRepository.ts │ ◂ AuthRoutes.ts    │
│ Símbolos: loginUser(), logout(), TokenManager                │
├──────────────────────────────────────────────────────────────┤
│ 📊 342 archivos │ 🔗 1200 dependencias │ ⏱ Escaneo: 3.4s   │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Top Bar
- Nombre de proyecto activo.
- Campo de búsqueda de nodos (autocomplete, resultados en dropdown).
- Badge de estado: `✅ Listo` | `🔄 Escaneando...` | `❌ Error`.
- Settings gear + window controls (Tauri).

### 3.3 Explorer (izquierda)
- Árbol de archivos por carpeta.
- Iconos por extensión.
- Scroll independiente.
- Búsqueda local (filtra el árbol).
- Archivo seleccionado → highlight + foco de nodo en grafo.

### 3.4 Grafo (centro)
- React Flow canvas con nodos coloreados por tipo.
- Controles: zoom (+/-/fit), minimapa (esquina).
- Click en nodo → selección (highlight azul), panel inferior y IA se actualizan.
- Hover en nodo → tooltip: nombre, tipo, # dependencias.
- Hover en arista → highlight de origen/destino.
- Drag de nodo permitido (no persiste layout en v1).

**Leyenda (flotante, colapsable):**
- 🟦 Route | 🟩 Service | 🟨 Repository | 🟪 Model | ⬜ External

### 3.5 Panel IA (derecha)
- **Modo Explain:** al hacer clic en "Explicar este archivo", respuesta Markdown con:
  - Summary (1-2 frases en bold).
  - Detalles (Markdown completo).
  - Rol en arquitectura.
- **Modo Chat:** historial de mensajes (scrollable), input abajo.
  - Respuestas del asistente en burbuja izquierda, usuario en derecha.
  - Loading: "..." animado mientras espera IA.
  - Error: mensaje rojo con acción sugerida.
  - Referencias a nodos: links clickeables que enfocan nodo en grafo.

### 3.6 Panel inferior (detalles)
Visible cuando hay nodo seleccionado. Colapsable.
- **Header:** nombre de archivo + tipo + badge.
- **Sección Símbolos:** tabla (nombre, kind, línea, export).
- **Sección Dependencias:** lista de archivos que importa → click focus.
- **Sección Dependientes:** lista de archivos que lo importan → click focus.

### 3.7 Status Bar
- 📊 count de archivos.
- 🔗 count de dependencias.
- ⏱ duración del escaneo.

---

## 4. Pantalla 3 — Settings

### 4.1 Layout
```
┌────────────────────────────┐
│ Settings               ✕  │
├────────────────────────────┤
│ IA                          │
│ Provider:  [Anthropic  ▾]  │
│ API Key:   [••••••••••  ]  │
│ Model:     [MiniMax M1 ▾]  │
│ [Test Connection]          │
│                            │
│ General                    │
│ Theme:     [Dark  ▾]      │
│ Language:  [Español ▾]    │
│                            │
│ Data                       │
│ [Clear cached projects]   │
│ [Export logs]              │
│                            │
│ Versión 1.0.0              │
└────────────────────────────┘
```

---

## 5. Estados de cada panel

### 5.1 Explorer
| Estado | Visual |
|---|---|
| Loading | Skeleton tree (3 niveles, 5-7 items grises animados). |
| Empty | "No se encontraron archivos. ¿Seleccionaste la carpeta correcta?" |
| Error | "Error al cargar archivos. [Reintentar]" |
| Ready | Árbol completo con scroll. |

### 5.2 Grafo
| Estado | Visual |
|---|---|
| Loading | Canvas gris con spinner central + "Construyendo grafo..." |
| Empty | Canvas con mensaje: "No se detectaron dependencias en este proyecto." |
| Error | Canvas con mensaje de error + botón reintentar. |
| Ready | Nodos y aristas visibles. |
| Large (>1000 nodos visibles) | Warning toast: "Grafo grande detectado. Usá búsqueda y filtros para navegar." |

### 5.3 IA Panel
| Estado | Visual |
|---|---|
| Sin API key | Placeholder con link a Settings. |
| Sin nodo seleccionado | "Seleccioná un archivo en el grafo o explorer para explicarlo." |
| Loading explain | Skeleton de respuesta con shimmer. |
| Loading chat | "..." animado en burbuja del asistente. |
| Error | Mensaje específico (timeout, rate limit, auth) con acción. |
| Ready | Respuesta renderizada en Markdown. |

### 5.4 Panel inferior
| Estado | Visual |
|---|---|
| Sin nodo seleccionado | Oculto o colapsado. |
| Loading | Skeleton con 3 secciones. |
| Ready | Metadata visible. |

---

## 6. Transiciones y animaciones

| Transición | Duración | Easing |
|---|---|---|
| Panel inferior expand/colapse | 200ms | ease-in-out |
| Selección de nodo (highlight) | 150ms | ease-out |
| Apertura de settings modal | 200ms | ease-out |
| Skeleton → contenido | 300ms fade | ease-in |
| Toast notificación | 300ms slide-in + auto-dismiss 5s | ease-out |

---

## 7. Responsive y accesibilidad

### 7.1 Mínimo soportado
- **Resolución mínima:** 1280×720.
- **Óptimo:** 1920×1080.
- La app es desktop-only (Tauri). No responsive mobile.

### 7.2 Accesibilidad
- Todo elemento interactivo es focusable con Tab.
- Contraste mínimo WCAG AA (4.5:1 para texto normal).
- Labels en íconos con `aria-label`.
- Estados de error comunicados con color + texto (no solo color).

---

## 8. Supuestos UX no resueltos

| # | Supuesto | Dueño | Target |
|---|---|---|---|
| U1 | Layout de 3 columnas fijo. En pantallas < 1280px se colapsa sidebar derecha | UX | Sprint 3 |
| U2 | Drag & drop de nodos persiste layout en v2 | UX | v2 planning |
| U3 | Panel inferior reemplaza grafo parcialmente → ¿mejor como overlay? | UX | Sprint 3 demo |

---

*Documento pre-SDD. Validar con mockups interactivos durante Sprint 3.*
