# CodeAtlas — Especificación IA: Prompts, Límites, Costos

**Versión:** pre-SDD v1
**Alcance:** diseño concreto de prompts, ventanas de tokens, costos y estrategia de fallback

---

## 1. Arquitectura de capa IA

```
Frontend (React)                    Backend (Rust)
     │                                    │
     │  invoke("explain_node", ...)      │
     ├──────────────────────────────────►│
     │                                    ├─ AIProvider trait
     │                                    ├─ AnthropicProvider
     │                                    │   └─ POST /v1/messages
     │                                    ├─ ContextBuilder
     │                                    │   └─ file + deps + metadata
     │                                    └─ ResponseParser
     │                                    │
     │  NodeExplanation                  │
     │◄──────────────────────────────────┤
```

**Proveedor v1:** Anthropic (único).  
**Modelo v1:** MiniMax (primer modelo operativo).  
**Capa de abstracción:** `trait AIProvider` permite cambiar modelo/proveedor sin tocar frontend.

---

## 2. Trait `AIProvider` (Rust)

```rust
#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn explain(&self, context: &NodeContext) -> Result<NodeExplanation, AIError>;
    async fn chat(&self, context: &ChatContext, history: &[ChatMessage]) -> Result<ChatResponse, AIError>;
    fn validate_api_key(&self, key: &str) -> Result<bool, AIError>;
}

pub enum AIError {
    Timeout,
    RateLimited { retry_after_secs: u64 },
    Unauthorized,
    NetworkError(String),
    TokenLimitExceeded { current: usize, max: usize },
    EmptyResponse,
    ParseError(String),
}
```

---

## 3. Prompts

### 3.1 System prompt base (compartido)

```
Eres CodeAtlas Assistant, un experto en arquitectura de software que ayuda a desarrolladores
a entender código. Trabajás dentro de una aplicación de escritorio llamada CodeAtlas.

Reglas:
- Respondé en el mismo idioma de la pregunta (español o inglés).
- Usá Markdown para estructurar respuestas largas.
- Si no tenés suficiente contexto para responder con certeza, decilo explícitamente.
- No inventes nombres de archivos, imports o dependencias que no estén en el contexto.
- Citá archivos específicos cuando estén en el contexto (ej: "En `AuthService.ts`...").
- Mantené respuestas concisas pero completas.
```

### 3.2 Prompt `explain_node`

**System addendum:**
```
Estás explicando un archivo específico del proyecto. El usuario quiere entender:
- Qué hace este archivo o módulo.
- Cuál es su rol en la arquitectura.
- Qué dependencias clave tiene.
```

**User message template:**
```
## Archivo a explicar
**Nombre:** {file_name}
**Path:** {file_path}
**Tipo detectado:** {node_type}
**Símbolos principales:** 
{symbols_summary}

## Contenido del archivo
```{extension}
{file_content_truncated}
```

## Dependencias inmediatas
{dependencies_summary}

## Contexto del proyecto
- Total de archivos: {total_files}
- Dependencias detectadas para este archivo: {dep_count}

Explicá este archivo.
```

### 3.3 Prompt `chat`

**System addendum:**
```
Estás en modo chat contextual con el proyecto. El usuario puede preguntar sobre:
- Cómo funciona un flujo específico.
- Relaciones entre archivos/módulos.
- Arquitectura general del proyecto.
- Mejores prácticas observadas.

Tenés acceso a los siguientes archivos como contexto inmediato:
{context_files_summary}

Si la pregunta requiere información fuera del contexto, indicalo y sugerí cómo obtenerla.
```

**User message template:**
```
## Pregunta del usuario
{user_message}

## Contexto relevante
{context_window}
```

---

## 4. Construcción de contexto

### 4.1 `NodeContext` (para explain)
```rust
pub struct NodeContext {
    pub file: FileContext,           // archivo principal
    pub dependencies: Vec<FileContext>,  // top-5 archivos importados
    pub dependents: Vec<FileContext>,    // top-3 archivos que lo importan
    pub project_stats: ProjectStats,     // files_count, total_deps
}

pub struct FileContext {
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub symbols: Vec<SymbolSummary>,
    pub content: String,            // truncado a 8KB
    pub extension: String,
}
```

### 4.2 `ChatContext` (para chat)
```rust
pub struct ChatContext {
    pub focus_nodes: Vec<FileContext>,   // nodos seleccionados o mencionados
    pub project_stats: ProjectStats,
    pub architecture_summary: Option<String>,  // resumen generado por IA post-scan
}
```

### 4.3 Reglas de truncado
| Campo | Límite | Estrategia |
|---|---|---|
| `file.content` | 8 KB | Truncar a 8KB, agregar `[truncated]` al final. |
| Dependencias | Top 5 | Ordenadas por relevancia (más imports compartidos). |
| Dependientes | Top 3 | Ordenados por cantidad de imports. |
| Historial de chat | Últimos 10 mensajes | Descartar los más antiguos. |
| System prompt + user message | ≤ 100K tokens totales | Si excede, reducir contexto de archivos. |

---

## 5. Límites de tokens y costos

### 5.1 Presupuesto por request (MiniMax via Anthropic API)
| Request type | Max tokens in (context) | Max tokens out (response) | Costo estimado |
|---|---|---|---|
| `explain_node` | 8,000 | 1,000 | ~$0.02 |
| `chat` (single turn) | 12,000 | 2,000 | ~$0.04 |
| `architecture_summary` (post-scan) | 20,000 | 3,000 | ~$0.08 |

### 5.2 Estrategia de control de costos
- **Rate limit local:** máximo 20 requests/minuto. Si se excede, UX muestra "esperá N segundos".
- **Contador de tokens por sesión:** visible en status bar o panel de diagnóstico (v2).
- **Presupuesto diario configurable:** default 500 requests/día. Al alcanzar, UX avisa.
- **Sin auto-retry en rate limit del proveedor:** backoff exponencial con jitter.

### 5.3 Modelos alternativos (future-proof)
| Modelo | Proveedor | Ventana contexto | Uso |
|---|---|---|---|
| MiniMax (m1) | Anthropic API | 100K tokens | Default v1 |
| Claude Sonnet | Anthropic API | 200K tokens | Upgrade path v1.1 |
| Claude Opus | Anthropic API | 200K tokens | v2 (análisis complejo) |

---

## 6. Fallback y resiliencia

### 6.1 Jerarquía de errores
```
1. Timeout (10s) → Retry 1 vez (exponential backoff 2s)
2. Rate limit (429) → Esperar Retry-After header, reintentar 1 vez
3. Auth error (401) → No reintentar, notificar UX
4. Network error → No reintentar, mostrar mensaje
5. Token limit → Internamente recortar contexto y reintentar (max 1 vez)
6. Empty response → No reintentar, mostrar "sin respuesta"
```

### 6.2 Circuit breaker (v2)
- Si 5 requests consecutivos fallan, pausar llamadas IA por 30s.
- UX muestra "IA no disponible temporalmente".
- Se reactiva automáticamente tras ventana de enfriamiento.

---

## 7. Seguridad del prompt

| Regla | Implementación |
|---|---|
| No incluir secrets en prompt | API key va por header HTTP, nunca en body |
| No incluir código fuente completo | Truncado a 8KB máximo |
| Sanitizar nombres de archivo | Escapar caracteres especiales en el prompt |
| Rate limit por proyecto | Máximo 100 requests/proyecto/día |

---

## 8. Validación de respuestas

### 8.1 Checks post-procesamiento
| Check | Acción si falla |
|---|---|
| Respuesta vacía | Retornar `EmptyResponse` error |
| Respuesta > 10KB | Truncar con marcador `[...]` |
| Referencias a archivos inexistentes | Log warning, no modificar respuesta |
| Respuesta en idioma incorrecto | Aceptar igual (el system prompt guía, no fuerza) |

### 8.2 Métrica de fidelidad (v2)
- > 90% de las referencias a archivos en la respuesta deben existir en el grafo del proyecto.
- Si < 90%, marcar respuesta como "low fidelity" en UX.

---

## 9. Supuestos no resueltos

| # | Supuesto | Dueño | Target |
|---|---|---|---|
| I1 | Anthropic API es compatible con MiniMax vía endpoint Anthropic | Backend | Sprint 0 |
| I2 | Límite de 20 req/min es suficiente para uso single-user | Producto | Validar en alpha |
| I3 | 8KB de archivo es suficiente para que la IA entienda contexto | Backend | Sprint 4 |
| I4 | Costo de ~$0.02 por explicación es aceptable para MVP | Producto | Aceptado |

---

*Documento pre-SDD. Refinar prompts con testing real durante Sprint 4.*
