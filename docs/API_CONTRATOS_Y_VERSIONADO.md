# CodeAtlas — Contratos API y Versionado

**Versión:** v1.0 (pre-SDD)
**Alcance:** v1 MVP + definición forward-compat para v2/v3

---

## 1. Política de versionado

### 1.1 Esquema
- **Formato:** `v{major}.{minor}` (ej: `v1.0`, `v1.1`, `v2.0`).
- **Major:** cambios que rompen compatibilidad (remoción de campos, cambio de tipo, nuevo campo obligatorio).
- **Minor:** adiciones backward-compat (nuevos campos opcionales, nuevos endpoints sin modificar existentes).
- **No se depreca sin aviso:** un endpoint/field obsoleto se marca `@deprecated` una minor version antes de removerlo.

### 1.2 Canal de transporte
- **Tauri `invoke` commands** entre frontend (TypeScript) y backend (Rust).
- Serialización: JSON vía `serde_json` en Rust ↔ tipos TypeScript sincronizados manualmente.
- Cada comando Tauri tiene firma documentada en `tauri-api.ts` y `engine/src/lib.rs`.

### 1.3 Regla backward compatibility
- Campos nuevos deben ser `Option<T>` en Rust / `field?` en TypeScript para no romper clientes.
- Nunca cambiar el tipo de un campo existente entre minor versions.
- Tests de contrato (JSON snapshots) protegen regresiones en CI.

---

## 2. Contratos v1 — Comandos Tauri

### 2.1 `scan_project`

**Request:**
```json
{
  "path": "/absolute/path/to/project"
}
```

**Response `ScanResult`:**
```json
{
  "project_id": "uuid-v4",
  "project_name": "my-app",
  "root_path": "/absolute/path/to/project",
  "files_count": 342,
  "symbols_count": 1890,
  "imports_count": 1200,
  "scan_duration_ms": 3450,
  "status": "ready",
  "error": null
}
```

**Estados posibles de `status`:**
- `"idle"` — sin escaneo.
- `"scanning"` — walker activo.
- `"building_graph"` — construyendo grafo tras scan.
- `"ready"` — completo.
- `"error"` — falló; `error` contiene mensaje.

**Errores:**
| Código | Significado |
|---|---|
| `PATH_NOT_FOUND` | Ruta no existe o no es carpeta. |
| `ACCESS_DENIED` | Sin permisos de lectura. |
| `SCAN_TIMEOUT` | >30s sin respuesta (proyecto enorme). |
| `INTERNAL` | Error inesperado en Rust. |

---

### 2.2 `get_scan_status`

**Request:**
```json
{ "projectId": "uuid" }
```

**Response:**
```json
{
  "status": "building_graph",
  "progress": 67.5,
  "files_processed": 230,
  "total_files": 342
}
```

---

### 2.3 `get_graph`

**Request:**
```json
{ "projectId": "uuid" }
```

**Response `GraphData`:**
```json
{
  "project_id": "uuid",
  "generated_at": "2026-05-31T20:00:00Z",
  "nodes": [
    {
      "id": "file-uuid",
      "label": "AuthService.ts",
      "path": "src/services/AuthService.ts",
      "type": "service",
      "symbol_count": 5,
      "position": { "x": 200, "y": 300 }
    }
  ],
  "edges": [
    {
      "id": "edge-uuid",
      "source": "file-uuid-a",
      "target": "file-uuid-b",
      "imports": ["login", "logout"]
    }
  ]
}
```

**`NodeType` enum (v1):**
- `component`, `route`, `service`, `repository`, `model`, `util`, `config`, `test`, `external`, `unknown`.

---

### 2.4 `get_node_details`

**Request:**
```json
{ "nodeId": "uuid" }
```

**Response `NodeDetails`:**
```json
{
  "file": {
    "id": "uuid",
    "path": "src/services/AuthService.ts",
    "name": "AuthService.ts",
    "extension": ".ts",
    "lines": 145,
    "content_hash": "sha256hex"
  },
  "symbols": [
    {
      "id": "sym-uuid",
      "name": "loginUser",
      "kind": "function",
      "line_start": 22,
      "line_end": 45,
      "is_exported": true
    }
  ],
  "dependencies": [
    { "node_id": "uuid", "label": "UserRepository.ts", "type": "repository" }
  ],
  "dependents": [
    { "node_id": "uuid", "label": "AuthRoutes.ts", "type": "route" }
  ],
  "node_type": "service"
}
```

---

### 2.5 `search_nodes`

**Request:**
```json
{
  "projectId": "uuid",
  "query": "auth",
  "limit": 20
}
```

**Response:**
```json
{
  "nodes": [
    { "id": "uuid", "label": "AuthService.ts", "path": "...", "type": "service" }
  ],
  "total_hits": 5
}
```

---

### 2.6 `explain_node`

**Request:**
```json
{
  "nodeId": "uuid",
  "symbolId": null
}
```

**Response `NodeExplanation`:**
```json
{
  "node_id": "uuid",
  "summary": "Servicio de autenticación que maneja login, logout y tokens JWT.",
  "details": "## AuthService\n\nEste archivo...\n\n### Funciones principales\n- `loginUser`: valida credenciales...",
  "dependencies_note": "Depende de UserRepository para persistencia.",
  "role": "service",
  "model_used": "anthropic-minimax",
  "tokens_used": 1240
}
```

---

### 2.7 `chat`

**Request:**
```json
{
  "projectId": "uuid",
  "message": "¿Cómo funciona el flujo de autenticación?",
  "history": [
    { "id": "msg-1", "role": "user", "content": "¿Qué es AuthService?", "timestamp": "..." },
    { "id": "msg-2", "role": "assistant", "content": "Servicio que...", "timestamp": "..." }
  ],
  "contextNodeIds": ["uuid-auth-service"]
}
```

**Response `ChatResponse`:**
```json
{
  "message": {
    "id": "msg-3",
    "role": "assistant",
    "content": "El flujo de autenticación inicia en AuthRoutes.ts...",
    "timestamp": "2026-05-31T20:01:00Z"
  },
  "referenced_nodes": ["uuid-auth-routes", "uuid-auth-service", "uuid-user-repo"],
  "model_used": "anthropic-minimax",
  "tokens_used": 2100
}
```

---

### 2.8 `configure_ai`

**Request:**
```json
{
  "config": {
    "provider": "anthropic",
    "api_key": "sk-ant-...",
    "model": "minimax-m1",
    "endpoint": null
  }
}
```

**Response:**
```json
{ "ok": true }
```

**Errores:**
| Código | Significado |
|---|---|
| `INVALID_KEY` | API key rechazada por proveedor. |
| `UNREACHABLE` | No se pudo conectar al endpoint. |

---

### 2.9 `get_ai_config`

**Request:**
```json
{}
```

**Response:**
```json
{
  "provider": "anthropic",
  "model": "minimax-m1",
  "configured": true
}
```

API key **nunca** se devuelve; queda solo en keyring del SO.

---

## 3. Eventos (Tauri events)

| Evento | Payload | Dirección |
|---|---|---|
| `scan:progress` | `{ status, progress, files_processed, total_files }` | BE → UI |
| `scan:complete` | `{ project_id, duration_ms }` | BE → UI |
| `scan:error` | `{ code, message }` | BE → UI |

---

## 4. Contratos forward-compat v2/v3

### 4.1 Extensiones planeadas v2
- `get_architecture_detection(projectId) → ArchitectureDetectionResult`
- `get_impact_analysis(nodeId, direction) → ImpactAnalysisResult`
- `get_graph_insights(projectId) → GraphInsights`
- `export_view(projectId, format) → binary/blob`

### 4.2 Extensiones planeadas v3
- `create_snapshot(projectId, label) → Snapshot`
- `add_comment(nodeId, text, author) → Comment`
- `share_view(viewId, recipients) → ShareLink`
- `get_health_timeline(projectId, from, to) → HealthScoreTimeline`

### 4.3 Regla de migración
- Endpoints v1 se mantienen funcionales hasta al menos v3.0.
- Nuevos campos son siempre opcionales hasta la siguiente major.
- `GraphData` v2 agrega `insights?: GraphInsights` (campo opcional, no rompe v1).
- `NodeType` se expande sin remover valores existentes.

---

## 5. Tests de contrato

### 5.1 Estructura
```
tests/contracts/
├── v1/
│   ├── scan_project.snap.json
│   ├── get_graph.snap.json
│   ├── get_node_details.snap.json
│   ├── explain_node.snap.json
│   └── chat.snap.json
└── README.md
```

### 5.2 CI check
- Cada PR que toca `engine/src/lib.rs` o `src/lib/tauri-api.ts` corre snapshots de contrato.
- Si el snapshot cambia, el PR debe actualizarlo explícitamente y documentar breaking change.

---

## 6. Supuestos no resueltos

| # | Supuesto | Dueño | Target |
|---|---|---|---|
| A1 | `NodeType` se infiere por heurísticas de path/nombre; ¿umbral de confianza? | Arquitectura | Sprint 2 |
| A2 | `progress` es % lineal; puede ser no lineal si el grafo tarda más que el scan | Backend | Sprint 2 |
| A3 | Tauri events en v1 o polling desde frontend cada 500ms | Tech Lead | Sprint 0 |

---

*Documento pre-SDD. Versión inicial. Se refinará durante fase `spec` del SDD v1.*
