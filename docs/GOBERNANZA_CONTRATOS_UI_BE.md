# CodeAtlas — Gobernanza de Contratos UI ↔ Backend

**Versión:** pre-SDD v1
**Alcance:** cómo definir, versionar, testear y evolucionar contratos sin romper nada

---

## 1. El contrato como source of truth

### 1.1 ¿Qué es un contrato?
La especificación **única y versionada** de cada comando Tauri, incluyendo:
- Nombre del comando.
- Request type (TypeScript interface + Rust struct).
- Response type (TypeScript interface + Rust struct).
- Errores posibles (variants).
- Comportamiento esperado (side effects, performance).

### 1.2 ¿Dónde vive?
```
src/lib/types.ts              ← TypeScript types (frontend)
src/lib/tauri-api.ts          ← invoke signatures (frontend)
engine/src/models/*.rs        ← Rust structs + serde derives (backend)
engine/src/lib.rs             ← #[tauri::command] registrations (backend)
tests/contracts/v1/*.snap.json ← JSON snapshots (contract tests)
```

---

## 2. Flujo de cambio de un contrato

### 2.1 Proceso estándar
```
1. Developer abre issue "Cambiar contrato: explain_node"
2. Se discute el cambio en el issue (breaking o no, alternativas)
3. Se aprueba → PR que modifica:
   a. Rust struct (con serde defaults para campos nuevos)
   b. TypeScript interface (con ? para campos opcionales)
   c. Contract test snapshot (actualizado)
   d. CHANGELOG.md en docs/
4. Reviewer valida:
   - Forward-compat: ¿un cliente viejos puede seguir usando esto?
   - Snapshot test pasa en CI
   - No se removieron campos sin deprecation previo
5. Merge → deploy
```

### 2.2 Breaking changes
Si un cambio **requiere** romper compatibilidad:

1. **Deprecar** en versión `N.minor`: agregar `@deprecated` en TS y `#[deprecated]` en Rust. El campo sigue funcionando.
2. **Esperar** una minor version completa.
3. **Remover** en `N+1.major`.

Ejemplo:
```typescript
// v1.2: deprecado
export interface ScanResult {
  /** @deprecated Usar scan_duration_ms en su lugar. Se removerá en v2.0 */
  duration_seconds?: number;
  scan_duration_ms: number;
}
```

```rust
// v1.2: deprecado
#[derive(Serialize)]
pub struct ScanResult {
    #[deprecated(note = "Usar scan_duration_ms. Se removerá en v2.0")]
    pub duration_seconds: Option<f64>,
    pub scan_duration_ms: u64,
}
```

---

## 3. Versiones de contrato

### 3.1 Contract version vs App version
- **App version:** `v1.2.3` (semver del producto).
- **Contract version:** `v1` (major de API).
- Un cambio de `Contract version` solo ocurre cuando hay breaking changes irreversibles.

### 3.2 Registry central
```typescript
// src/lib/contract-versions.ts
export const API_VERSION = 1;

export const COMMANDS = {
  scan_project: { version: 1, deprecated: false },
  get_graph: { version: 1, deprecated: false },
  get_node_details: { version: 1, deprecated: false },
  search_nodes: { version: 1, deprecated: false },
  explain_node: { version: 1, deprecated: false },
  chat: { version: 1, deprecated: false },
  configure_ai: { version: 1, deprecated: false },
} as const;
```

---

## 4. Sincronización TypeScript ↔ Rust

### 4.1 Regla de oro
**Un cambio en un tipo DEBE reflejarse en ambos lados en el MISMO PR.**  
No se mergea un cambio de frontend sin su contraparte en backend (y viceversa).

### 4.2 Checklist de sincronización (por PR)
- [ ] `types.ts`: interface actualizada.
- [ ] `models/*.rs`: struct actualizada con `#[derive(Serialize, Deserialize)]`.
- [ ] Campos nuevos son `Option<T>` en Rust y `field?` en TS.
- [ ] Nombres de campos coinciden exactamente (serde `#[serde(rename_all = "camelCase")]`).
- [ ] Enums coinciden (TS string union ↔ Rust enum con `#[serde(rename_all = "snake_case")]`).
- [ ] Contract test snapshot actualizado.
- [ ] `tauri-api.ts`: firma de invoke coincide.

### 4.3 Serde configuration (Rust)
```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub symbol_count: u32,
    pub position: Option<Position>,
}
```

---

## 5. Contract tests

### 5.1 Propósito
Detectar cambios accidentales en la forma de los datos serializados.

### 5.2 Implementación
```rust
// tests/contracts/v1/scan_project_test.rs
use insta::assert_json_snapshot;

#[test]
fn contract_scan_project_response() {
    let result = simulate_scan("fixtures/simple-ts");
    assert_json_snapshot!("scan_project", result);
}
```

### 5.3 CI enforcement
- Contract tests corren en cada PR que toca `engine/src/models/` o `src/lib/types.ts`.
- Si snapshot cambia → el PR debe actualizarlo explícitamente.
- Reviewer verifica que el cambio es intencional y documentado.

---

## 6. Errores como parte del contrato

### 6.1 Formato de error estándar
```json
{
  "error": {
    "code": "SCAN_TIMEOUT",
    "message": "El escaneo excedió el tiempo límite de 30 segundos.",
    "details": {
      "files_processed": 4800,
      "total_files": 5000
    }
  }
}
```

### 6.2 Códigos de error estables
| Código | Significado | Versión |
|---|---|---|
| `PATH_NOT_FOUND` | Ruta no existe | v1 |
| `ACCESS_DENIED` | Sin permisos | v1 |
| `SCAN_TIMEOUT` | Timeout de escaneo | v1 |
| `INVALID_KEY` | API key inválida | v1 |
| `UNREACHABLE` | No se puede conectar al proveedor IA | v1 |
| `RATE_LIMITED` | Rate limit del proveedor | v1 |
| `TOKEN_LIMIT` | Contexto excede tokens | v1 |
| `INTERNAL` | Error inesperado | v1 |

**Regla:** nunca remover un código de error. Agregar nuevos es backward-compatible.

---

## 7. Gobernanza de cambios

### 7.1 Responsabilidades
| Rol | Responsabilidad |
|---|---|
| **Tech Lead** | Aprueba breaking changes. Dueño del registry de contratos. |
| **Backend Dev** | Implementa lado Rust, actualiza snapshots. |
| **Frontend Dev** | Implementa lado TS, actualiza tipos. |
| **Reviewer** | Verifica checklist de sincronización. |

### 7.2 Ceremonia de cambio de contrato
- **Frecuencia:** ad-hoc, cuando un feature lo requiere.
- **Duración:** discusión en issue + PR review.
- **Gate:** Tech Lead + 1 reviewer.

---

## 8. Changelog de contratos

```markdown
# Changelog de contratos API

## v1.1 (planificado)
- `scan_project`: agregar campo opcional `exclude_patterns: string[]`
- `GraphNode`: agregar campo opcional `group?: string`

## v1.0 (MVP)
- Comandos iniciales: scan_project, get_scan_status, get_graph, 
  get_node_details, search_nodes, explain_node, chat, configure_ai.
```

**Ubicación:** `docs/CHANGELOG_CONTRATOS.md`

---

## 9. Supuestos no resueltos

| # | Supuesto | Dueño | Target |
|---|---|---|---|
| G1 | `insta` crate funciona para contract tests en CI | Backend | Sprint 0 |
| G2 | No se necesita versionar el protocolo de comunicación (Tauri invoke es suficiente) | Tech Lead | Aceptado |
| G3 | Serde camelCase + TypeScript camelCase cubre todos los casos sin conflictos | Backend | Sprint 0 |

---

*Documento pre-SDD. Activar proceso de gobernanza desde Sprint 0.*
