# CodeAtlas — Observabilidad: Logs, Métricas y Tracing

**Versión:** pre-SDD v1
**Alcance:** estrategia de observabilidad para debug, monitoreo y mejora de performance

---

## 1. Principios

| Principio | Descripción |
|---|---|
| **Nunca loguear código fuente de usuario** | Seguridad ante todo. |
| **Logs estructurados** | JSON o key-value, no texto libre. |
| **Levels consistentes** | ERROR > WARN > INFO > DEBUG > TRACE. |
| **Métricas para decisiones** | Cada métrica responde una pregunta de producto o performance. |
| **Cero dependencia de servicios cloud** | Todo local. Sin SaaS de observabilidad. |

---

## 2. Niveles de log

| Nivel | Uso | Ejemplo |
|---|---|---|
| `ERROR` | Fallos que impiden función core | "Failed to parse file: path=%s, reason=syntax error" |
| `WARN` | Degradación no crítica | "Scan timeout approaching: files=4800, elapsed=9.2s" |
| `INFO` | Eventos de ciclo de vida | "Scan completed: project=%s, files=%d, duration=%dms" |
| `DEBUG` | Detalle para desarrollo | "Resolved import: from=%s, to=%s, alias=%s" |
| `TRACE` | Máximo detalle (no en prod) | "Walker visited: path=%s, depth=%d" |

---

## 3. Implementación

### 3.1 Backend (Rust)
- **Crate:** `tracing` + `tracing-subscriber`.
- **Output:** archivo rotativo en `~/.local/share/codeatlas/logs/` (Linux), equivalente en macOS/Windows.
- **Formato:** JSON lines (`.jsonl`).
- **Rotación:** diaria, mantener últimos 7 días. Máximo 10 MB por archivo.

```rust
// engine/src/lib.rs
use tracing::{info, warn, error, debug};

pub fn init_logging(data_dir: &Path) {
    let file_appender = tracing_appender::rolling::daily(data_dir.join("logs"), "codeatlas");
    tracing_subscriber::fmt()
        .json()
        .with_writer(file_appender)
        .with_max_level(if cfg!(debug_assertions) { Level::DEBUG } else { Level::INFO })
        .init();
}
```

### 3.2 Frontend (TypeScript)
- **Consola del navegador** + Tauri `invoke` para logs críticos.
- En producción (Tauri release build), logs van a `stdout/stderr` capturados por el proceso Tauri.
- No se persisten logs de frontend en disco por defecto.

### 3.3 Estructura de entrada de log
```json
{
  "timestamp": "2026-05-31T20:00:00.123Z",
  "level": "INFO",
  "target": "codeatlas::scanner::walker",
  "message": "Scan started",
  "project_id": "uuid",
  "fields": {
    "root_path": "/home/user/project",
    "extensions": [".ts", ".tsx", ".js", ".jsx"]
  }
}
```

---

## 4. Métricas clave

### 4.1 Métricas de producto (qué mide el éxito)
| Métrica | Pregunta | Fuente | Frecuencia |
|---|---|---|---|
| `scan.duration_ms` | ¿Es rápido el escaneo? | Rust timer | Por escaneo |
| `graph.render_time_ms` | ¿El grafo se ve rápido? | Frontend performance API | Por render |
| `ai.response_time_ms` | ¿La IA responde en tiempo? | Rust timer + network | Por request IA |
| `ai.tokens_per_request` | ¿Cuánto cuesta cada respuesta? | API response metadata | Por request IA |

### 4.2 Métricas de sistema (salud técnica)
| Métrica | Pregunta | Fuente |
|---|---|---|
| `memory.rss_mb` | ¿Estamos dentro de budget? | `sysinfo` crate |
| `files.scanned_total` | ¿Carga de trabajo? | Scanner counter |
| `errors.scan_failures` | ¿Tasa de fallos? | Contador de errores |
| `errors.ai_timeouts` | ¿Problemas de red/IA? | Contador de errores |

### 4.3 Exposición
- Las métricas se escriben como logs estructurados con `target: "codeatlas::metrics"`.
- En v2: endpoint Tauri `get_metrics()` para mostrar en panel de diagnóstico.
- En v3: exportables como JSON para análisis externo.

---

## 5. Tracing de requests IA

### 5.1 Span hierarchy
```
ai:explain_node
├── ai:build_context (cuánto tarda construir el contexto)
├── ai:call_api (request HTTP)
│   ├── tokens_in, tokens_out
│   └── latency_ms
└── ai:parse_response
```

### 5.2 Qué se registra
- `project_id`, `node_id` (sin contenido de archivo).
- `model`, `provider`.
- `tokens_in`, `tokens_out`, `latency_ms`.
- `status`: success | timeout | rate_limited | error.
- **Nunca:** prompt completo, response completa, código fuente.

---

## 6. Manejo de errores IA

| Error | Log level | UX | Recuperación |
|---|---|---|---|
| Timeout (>10s) | WARN | "La IA está tardando. ¿Reintentar?" | Retry 1 vez |
| Rate limit (429) | WARN | "Límite de consultas alcanzado. Esperá %d segundos." | Esperar + retry |
| Auth error (401) | ERROR | "API key inválida. Verificá tu configuración." | Redirigir a settings |
| Network error | ERROR | "Sin conexión. Verificá tu red." | Sin retry automático |
| Empty response | WARN | "La IA no pudo generar una respuesta." | Sugerir reformular |
| Token limit exceeded | WARN | Recortar contexto y reintentar | Interno, transparente |

---

## 7. Panel de diagnóstico (v2)

En v2 se agrega una vista de diagnóstico accesible desde Settings > Diagnostics:

- Últimos 50 requests IA con latencia y tokens.
- Tiempo de último escaneo.
- Memoria actual.
- Errores en las últimas 24h.
- Botón "Exportar logs" (últimos 7 días como `.zip`).

---

## 8. Configuración

### 8.1 Variables de entorno
| Variable | Default | Propósito |
|---|---|---|
| `CODEATLAS_LOG_LEVEL` | `info` | Nivel mínimo de log |
| `CODEATLAS_LOG_DIR` | `~/.local/share/codeatlas/logs` | Directorio de logs |
| `CODEATLAS_METRICS_ENABLED` | `true` | Activar/desactivar métricas |

### 8.2 Tauri config
```json
// tauri.conf.json (sección app)
{
  "app": {
    "windows": [],
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "log": {
      "targets": ["codeatlas"]
    }
  }
}
```

---

## 9. Supuestos no resueltos

| # | Supuesto | Dueño | Target |
|---|---|---|---|
| O1 | `tracing-subscriber` funciona sin conflicto con Tauri runtime | Backend | Sprint 0 |
| O2 | Rotación de logs no bloquea el event loop de Tauri | Backend | Sprint 1 |
| O3 | Panel de diagnóstico en v2 no requiere persistencia adicional de métricas | Tech Lead | v2 planning |

---

*Documento pre-SDD. Implementar en Sprint 0 (init logging) y refinar en Sprint 5 (diagnostics).*
