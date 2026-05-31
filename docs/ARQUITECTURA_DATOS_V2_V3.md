# CodeAtlas — Arquitectura de Datos v2/v3

**Versión:** pre-SDD
**Alcance:** diseño forward-compat de migraciones, snapshots y entidades colaborativas

---

## 1. Principios de evolución del schema

| Principio | Descripción |
|---|---|
| **Additive-only en minor** | Nuevas tablas y columnas opcionales, sin borrar ni renombrar sin major. |
| **Migration scripts versionados** | Cada cambio de schema tiene un script `.sql` numerado en `engine/migrations/`. |
| **SQLite como motor único v1-v3** | Sin Postgres, sin SQL server. Para colaboración v3 se evalúa SQLite + CRDT o backend ligero. |
| **Cache en cliente, source of truth en SQLite** | Grafo se cachea en `graph_cache`, pero se reconstruye desde `files + imports` si hay cambios. |

---

## 2. Schema v1 (base — ya definido en Master Prompt)

```sql
projects(id, name, root_path, files_count, symbols_count, imports_count, scan_duration_ms, status, error, created_at, updated_at)
files(id, project_id FK, path, name, extension, lines, content_hash, parsed_at)
symbols(id, file_id FK, name, kind, line_start, line_end, is_exported)
imports(id, source_file_id FK, target_file_id FK?, target_module, import_names, is_default, is_type_import)
graph_cache(project_id PK FK, graph_json, generated_at)
ai_config(id=1 PK, provider, api_key_encrypted, model, endpoint, updated_at)
```

---

## 3. Migración v1.1 (chat + settings)

### 3.1 Nueva tabla: `chat_sessions`
```sql
CREATE TABLE chat_sessions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 3.2 Nueva tabla: `chat_messages`
```sql
CREATE TABLE chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
  content TEXT NOT NULL,
  referenced_nodes TEXT,  -- JSON array
  tokens_used INTEGER,
  model TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_chat_messages_session ON chat_messages(session_id, created_at);
```

### 3.3 Nueva tabla: `user_settings`
```sql
CREATE TABLE user_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
-- Ej: theme=dark, scan_defaults='{"exclude": ["node_modules"]}'
```

---

## 4. Schema v2 — Insights y Arquitectura

### 4.1 Nueva tabla: `architecture_detections`
```sql
CREATE TABLE architecture_detections (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  pattern TEXT NOT NULL,          -- 'mvc', 'layered', 'clean', 'hexagonal', 'unknown'
  confidence REAL NOT NULL,       -- 0.0 a 1.0
  evidence TEXT,                  -- JSON con nodos/aristas que respaldan
  detected_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_arch_detect_project ON architecture_detections(project_id);
```

### 4.2 Nueva tabla: `graph_insights`
```sql
CREATE TABLE graph_insights (
  project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  cycles TEXT,                    -- JSON array de ciclos [{nodes: [...], length: N}]
  hotspots TEXT,                  -- JSON array de [{node_id, coupling_score, reason}]
  avg_coupling REAL,
  density REAL,
  generated_at TEXT NOT NULL
);
```

### 4.3 Extensión de `imports` para usage edges (v2)
```sql
ALTER TABLE imports ADD COLUMN edge_type TEXT DEFAULT 'import';
-- edge_type: 'import' (v1) | 'call' | 'usage' | 'extend' | 'implement' (v2)
-- Los edges v1 quedan como 'import', nuevos edges de llamadas usan otros valores.
```

### 4.4 Migración sin downtime
- `ALTER TABLE imports ADD COLUMN` es seguro en SQLite (no bloquea escrituras concurrentes en WAL mode).
- La app detecta esquema viejo, corre migraciones automáticamente al iniciar.

---

## 5. Schema v3 — Colaboración y Multi-proyecto

### 5.1 Workspace (multi-proyecto)
```sql
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE workspace_projects (
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  added_at TEXT NOT NULL,
  PRIMARY KEY (workspace_id, project_id)
);
```

### 5.2 Snapshots de arquitectura
```sql
CREATE TABLE snapshots (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  description TEXT,
  graph_json TEXT NOT NULL,        -- GraphData completo congelado
  insights_json TEXT,              -- GraphInsights del momento
  created_by TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_snapshots_project ON snapshots(project_id, created_at);
```

### 5.3 Comentarios y anotaciones
```sql
CREATE TABLE annotations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  node_id TEXT NOT NULL,
  author TEXT NOT NULL,
  text TEXT NOT NULL,
  annotation_type TEXT DEFAULT 'comment',  -- comment | todo | review | issue
  resolved BOOLEAN DEFAULT FALSE,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_annotations_node ON annotations(project_id, node_id);
```

### 5.4 Health timeline (v3)
```sql
CREATE TABLE health_records (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  snapshot_id TEXT REFERENCES snapshots(id) ON DELETE SET NULL,
  overall_score REAL,               -- 0.0 a 1.0
  coupling_score REAL,
  complexity_score REAL,
  cycle_count INTEGER,
  hotspot_count INTEGER,
  details TEXT,                     -- JSON con desglose
  recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_health_project ON health_records(project_id, recorded_at);
```

---

## 6. Estrategia de migraciones

### 6.1 Directorio
```
engine/migrations/
├── 001_initial_schema.sql
├── 002_chat_and_settings.sql
├── 003_architecture_and_insights.sql
├── 004_workspace_and_snapshots.sql
├── 005_collaboration_annotations.sql
└── 006_health_timeline.sql
```

### 6.2 Formato migration
```sql
-- Migration: 003_architecture_and_insights
-- Version: v2.0
-- Applied by: engine on startup if schema_version < 3

BEGIN;
ALTER TABLE imports ADD COLUMN edge_type TEXT DEFAULT 'import';
CREATE TABLE IF NOT EXISTS architecture_detections (...);
CREATE TABLE IF NOT EXISTS graph_insights (...);
PRAGMA user_version = 3;
COMMIT;
```

### 6.3 Rollback
- SQLite no soporta `DROP COLUMN` en versiones antiguas.
- Estrategia: migraciones forward-only. Si un cambio es riesgoso, se testea con backup de DB previo.
- Para rollback real, restaurar backup y re-ejecutar migraciones hasta el punto deseado.

---

## 7. Límites y decisiones abiertas

| # | Tema | Estado | Dueño | Target |
|---|---|---|---|---|
| D1 | ¿Colaboración v3 requiere backend server o todo local? | Abierto | Arquitectura | Durante v2 |
| D2 | ¿CRDT para merge de snapshots distribuidos o single-writer? | Abierto | Arquitectura | Durante v2 |
| D3 | Tamaño máximo de `graph_json` en SQLite (~1GB límite) | Mitigado: compresión zstd antes de guardar | Backend | Sprint 2 |
| D4 | ¿Health records se generan on-scan o bajo demanda? | On-demand con cache | Producto | v2 planning |

---

## 8. Supuestos de diseño

1. **SQLite escala para proyecto de hasta 50k archivos** con índices adecuados.
2. **WAL mode** activado por defecto para permitir lecturas concurrentes.
3. **graph_json comprimido** con zstd si supera 1MB crudo.
4. **Colaboración v3 es local-first**: los snapshots se exportan/importan; no hay servidor central.
5. **Un solo usuario escribe** la DB a la vez (app single-user desktop).

---

*Documento pre-SDD. Sujeto a refinamiento en fase `design` del SDD.*
