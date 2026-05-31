# CodeAtlas — Seguridad y Permisos v3

**Versión:** pre-SDD
**Alcance:** modelo de seguridad para features colaborativas (v3) + seguridad base v1

---

## 1. Postura de seguridad general

| Capa | Principio |
|---|---|
| **Proyecto escaneado** | Untrusted input. Solo análisis estático. Nunca ejecutar. |
| **Datos locales** | SQLite en espacio de usuario. Solo la app accede. |
| **Red** | Solo saliente: llamadas a API de IA. Sin servidor entrante en v1/v2. |
| **Credenciales** | API keys en keyring nativo del SO. Nunca en texto plano en DB. |
| **Colaboración (v3)** | Local-first. Snapshots exportables. Sin auth server. |

---

## 2. Seguridad v1 (base)

### 2.1 Análisis estático seguro
- El scanner **lee** archivos. Nunca ejecuta, evalúa, ni interpreta código.
- Tree-sitter es un parser, no un runtime. No ejecuta nada.
- Path traversal prevention: el walker rechaza paths que escapen del root del proyecto.
- Symlinks: no se siguen symlinks a menos que apunten dentro del root del proyecto.

### 2.2 Almacenamiento de API key
- **keyring del SO**:
  - Linux: `Secret Service API` (libsecret / gnome-keyring)
  - macOS: `Keychain Services`
  - Windows: `Credential Manager`
- La DB solo guarda metadata: `provider`, `model`, `endpoint`. `api_key_encrypted` guarda una referencia al keyring, no la key.
- Al iniciar: si no hay key en keyring, se pide al usuario.

### 2.3 Datos enviados a IA
- **Regla de mínimo privilegio:** solo se envía el archivo consultado + dependencias inmediatas + metadata del grafo.
- Nunca el proyecto completo.
- El código del archivo se trunca a 8KB si es muy grande. El contexto de dependencias se limita a top-5 imports.
- Logs del motor de IA registran tokens usados y modelo, pero **nunca** el contenido enviado/recibido.

### 2.4 Sandbox de Tauri
- Tauri v2 CSP (Content Security Policy) bloquea scripts externos en webview.
- `tauri.conf.json` → `capabilities/default.json` define permisos explícitos:
  - `fs:read` — solo para scanner (paths dentro del proyecto).
  - `dialog:open` — solo para seleccionar carpeta.
  - `shell:open` — deshabilitado en MVP.
  - `http:fetch` — solo para endpoint de IA configurado.

---

## 3. Seguridad v2 (features avanzadas)

Sin cambios de modelo de seguridad respecto a v1 porque:
- Export de vistas opera sobre datos ya locales, sin envío externo.
- Detección de arquitectura es local, sin red.
- Ciclos/hotspots son cálculo local.

Única adición: validación de `edge_type` en imports para prevenir inyección vía archivos escaneados maliciosos.

---

## 4. Seguridad y permisos v3 (colaboración)

### 4.1 Modelo conceptual
- **Local-first, no multi-tenant cloud.**
- No hay autenticación de usuarios en la app.
- La colaboración ocurre mediante **exportación/importación de snapshots** (archivos `.codeatlas-snapshot`).
- Los comentarios y anotaciones son locales a la instalación. Compartir es opcional y explícito.

### 4.2 Formato de snapshot (seguro)
```json
{
  "version": "3.0",
  "project_id": "uuid",
  "label": "Sprint 5 architecture",
  "created_at": "2026-06-15T10:00:00Z",
  "created_by": "local-user",
  "graph": { ... },
  "insights": { ... },
  "annotations": [...],
  "hash": "sha256-of-content"
}
```
- El hash permite verificar integridad al importar.
- Los snapshots no contienen código fuente de archivos (solo metadata y grafo).
- Al importar, se valida el hash y se muestra un preview antes de aplicar.

### 4.3 Permisos de colaboración
| Acción | Permiso requerido |
|---|---|
| Exportar snapshot | Ninguno (datos propios) |
| Importar snapshot | Confirmación explícita del usuario |
| Compartir snapshot | Fuera de la app (email, git, Slack, etc.) |
| Ver comentarios importados | Automático al importar snapshot |

### 4.4 Riesgos mitigados
| Riesgo | Mitigación |
|---|---|
| Snapshot malicioso (JSON injection) | Validación estricta de esquema + hash check. |
| Snapshot con datos inflados (DoS) | Límite de 50MB por snapshot. |
| Leak de código fuente via snapshot | Snapshots no incluyen código, solo metadata. |
| Modificación no autorizada de comentarios | Comentarios locales; autor es string libre. Sin auth server. |

---

## 5. Checklist de seguridad por versión

| Check | v1 | v2 | v3 |
|---|---|---|---|
| Path traversal prevention | ✅ | ✅ | ✅ |
| API key en keyring del SO | ✅ | ✅ | ✅ |
| CSP en webview | ✅ | ✅ | ✅ |
| Nunca ejecutar código escaneado | ✅ | ✅ | ✅ |
| Contexto IA mínimo | ✅ | ✅ | ✅ |
| Validación de esquema en importación | N/A | N/A | ✅ |
| Hash de integridad en snapshots | N/A | N/A | ✅ |
| Rate limiting de IA | ✅ | ✅ | ✅ |

---

## 6. Supuestos no resueltos

| # | Supuesto | Estado |
|---|---|---|
| S1 | La app es single-user. No se necesita modelo de roles. | Aceptado |
| S2 | Colaboración no requiere backend cloud. Si cambia en v3, el modelo de auth debe rediseñarse. | Riesgo registrado |
| S3 | El keyring del SO está disponible en todas las plataformas. En headless Linux puede fallar; se necesita fallback cifrado local. | Riesgo: verificar en CI |
| S4 | No se persisten logs de contenido enviado a IA. Si se requiere audit trail en v3, debe ser opt-in. | Aceptado |

---

*Documento pre-SDD. Refinar durante fase `spec` para features v3.*
