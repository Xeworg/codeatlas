# Plan: Outline con Tree-sitter y parsers reutilizables

Este documento propone convertir el análisis actual basado en Tree-sitter en una base extensible para mostrar una vista tipo **Outline de VS Code** dentro de CodeAtlas. La meta es pasar de un grafo plano de archivos a una navegación semántica por símbolos, preparada para sumar más lenguajes sin duplicar lógica.

La decisión central es que el outline no debería ser solo una mejora visual. Debería convertirse en un **índice semántico de código** que también pueda usar la IA para entender archivos grandes sin leerlos completos ni depender de truncados por bytes.

## Decisión propuesta

Agregar una capa de parser por lenguaje con contratos estables y un modelo `OutlineItem` propio. Primero se debería implementar para TypeScript/Rust, y después extender a Python, Go u otros lenguajes mediante un registro de parsers.

`OutlineItem` debería ser el primer consumidor visible de una base más general: un índice semántico Tree-sitter-first. La UI lo usa para mostrar navegación tipo outline; la IA lo usa para seleccionar símbolos y rangos relevantes antes de pedir o leer código fuente.

## Por qué importa

La mejora visual del grafo ya permite navegar nodos, pero todavía se ve mucho contenido como:

- `Unknown`
- `0 symbols`
- nodos de archivo sin detalle interno
- dependencias difíciles de interpretar cuando hay muchos archivos

Tree-sitter ya está disponible y puede dar más valor si se usa para extraer estructura interna: clases, funciones, métodos, interfaces, constantes, módulos, imports y relaciones.

También reduce costo y ruido para IA. En vez de enviar un archivo completo o sus primeros kilobytes truncados, CodeAtlas puede enviar una representación compacta:

```text
File: src/services/UserService.ts
Outline:
  class UserService lines 10-95
    constructor lines 12-18
    getUser lines 20-42
    saveUser lines 44-70
Imports:
  UserRepository from ./UserRepository
Dependents:
  UserController.ts
```

Con esa estructura, la IA puede decidir qué símbolo necesita expandir y CodeAtlas puede entregar solo el rango exacto de líneas, no todo el archivo.

## Objetivos

- Mostrar un outline expandible por nodo/archivo.
- Reutilizar Tree-sitter para símbolos, imports y outline.
- Separar la lógica por lenguaje para facilitar extensiones futuras.
- Mejorar la clasificación de nodos (`Model`, `Component`, `Service`, `Unknown`, etc.).
- Preparar una base para navegación semántica y análisis más profundo.
- Permitir que la IA use estructura Tree-sitter para orientarse antes de leer código fuente.
- Reemplazar el contexto IA basado en truncado bruto por contexto semántico: outline + imports + dependencias + extractos dirigidos.

## No objetivos iniciales

- No implementar todos los lenguajes de golpe.
- No reemplazar el grafo actual.
- No agregar edición de código desde el outline en la primera versión.
- No depender de IA para extraer símbolos si Tree-sitter puede hacerlo de forma determinista.
- No intentar que la IA entienda todo el repositorio leyendo archivos completos cuando existe un índice semántico navegable.
- No implementar navegación/editor embebido completa en la primera versión.

## Modelo conceptual

```text
Archivo / nodo del grafo
└─ Outline
   ├─ class UserService
   │  ├─ constructor()
   │  ├─ getUser()
   │  └─ saveUser()
   ├─ interface UserDto
   ├─ function parseUser()
   └─ const USER_SCHEMA
```

## Modelo de datos sugerido

```ts
export interface OutlineItem {
  id: string
  fileId: string
  name: string
  kind:
    | 'class'
    | 'function'
    | 'method'
    | 'interface'
    | 'type'
    | 'enum'
    | 'const'
    | 'variable'
    | 'module'
    | 'field'
    | 'unknown'
  lineStart: number
  lineEnd: number
  columnStart?: number
  columnEnd?: number
  children: OutlineItem[]
}
```

`SymbolInfo` puede seguir existiendo para análisis y métricas, pero `OutlineItem` debería ser un modelo propio porque necesita jerarquía y estructura de UI.

`OutlineItem.kind` no tiene que ser idéntico a `SymbolKind`: puede ser un subconjunto o superconjunto intencional. El outline necesita granularidad de UI (`field`, `method`, `module`) que no siempre coincide con las categorías usadas para métricas (`Struct`, `Impl`, `ArrowFunction`, etc.). La reconciliación exacta debería definirse durante la fase de spec.

## Arquitectura propuesta

```text
engine/src/scanner/
├─ parser/
│  ├─ mod.rs
│  ├─ traits.rs          # contrato común
│  ├─ registry.rs        # extensión/lenguaje → parser
│  ├─ typescript.rs
│  ├─ rust.rs
│  └─ python.rs          # futuro
├─ walker.rs
└─ parser.rs             # migrar o reducir gradualmente
```

Contrato sugerido:

```rust
pub struct ParseResult {
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<ImportInfo>,
    pub outline: Vec<OutlineItem>,
}

pub trait LanguageParser {
    fn language_id(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];

    fn parse_all(&self, source: &str, path: &str, file_id: &str) -> ParseResult;
}
```

`parse_all()` debería ser el contrato preferido porque permite recorrer el AST una sola vez y producir símbolos, imports y outline desde la misma estructura. Si durante implementación se necesita compatibilidad incremental, se pueden mantener helpers internos `parse_symbols`, `parse_imports` y `parse_outline`, pero la API pública del parser debería tender a `ParseResult`.

## Flujo backend propuesto

```text
scan_project
  → FileWalker descubre archivos
  → ParserRegistry selecciona parser por extensión
  → parser.parse_symbols()
  → parser.parse_imports()
  → parser.parse_outline()
  → persistir files/symbols/imports/outline_items
  → get_graph devuelve nodos + edges
  → get_node_outline(file_id) devuelve outline jerárquico
```

El `ParserRegistry` debería reemplazar el acceso directo actual a `CodeParser::parse_file`. Puede vivir como servicio de scanner o ser inyectado en `AppState`, siempre evitando que `scan_project` conozca detalles internos de cada lenguaje.

Para la primera versión, `OutlineItem` puede generarse bajo demanda desde el archivo fuente y/o desde símbolos ya persistidos. Persistir una tabla dedicada `outline_items` debería decidirse solo si la latencia de re-parseo resulta inaceptable.

Recomendación inicial: persistir JSON por archivo si el scan ya está generando el outline. El patrón de lectura más probable es “dame el árbol completo de este archivo”, no “consultá cada hijo por separado”. Una tabla normalizada con `parent_id` puede quedar para una fase posterior si hacen falta búsquedas globales por símbolo.

```sql
CREATE TABLE IF NOT EXISTS outline_items (
    file_id TEXT PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
    outline_json TEXT NOT NULL,
    generated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

## Cambios de API sugeridos

### Nuevo comando Tauri

```rust
#[tauri::command]
pub async fn get_node_outline(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<OutlineItem>, String>
```

### Wrapper frontend

```ts
export async function getNodeOutline(fileId: string): Promise<OutlineItem[]> {
  return await invoke<OutlineItem[]>('get_node_outline', { fileId })
}
```

## Índice semántico para IA

El outline debería alimentar una capa interna que puede llamarse `SemanticCodeIndex` o equivalente. No necesita exponerse completa en la primera versión, pero sí conviene diseñar el modelo para soportarla.

Responsabilidades futuras:

1. devolver outline por archivo;
2. buscar símbolo por nombre o tipo;
3. devolver el rango exacto de un símbolo;
4. construir contexto compacto para IA;
5. expandir código bajo demanda solo para símbolos relevantes.

APIs posibles a futuro:

```text
get_node_outline(file_id)
get_symbol_source(file_id, outline_item_id)
search_symbols(query)
build_ai_context_for_node(file_id, mode)
```

Modos de contexto IA:

| Modo      | Contenido                                  | Uso                                  |
| --------- | ------------------------------------------ | ------------------------------------ |
| `summary` | outline + imports + dependencias           | explicación rápida y barata          |
| `focused` | summary + extractos de símbolos relevantes | preguntas específicas o debugging    |
| `full`    | fallback con archivo completo/truncado     | casos donde la estructura no alcanza |

El flujo deseado para IA es:

```text
pregunta del usuario
  → cargar outline/imports/dependencias
  → identificar símbolos candidatos
  → leer solo rangos necesarios
  → responder con contexto puntual
```

Esto evita que la IA tenga que leer archivos enteros y mejora la calidad del análisis porque conserva la estructura global incluso en archivos grandes.

## Integración con contexto IA actual

El punto principal de integración es `engine/src/ai/context.rs`. Hoy el contexto de nodo puede depender de contenido fuente truncado. La mejora debería construir primero una representación semántica:

```text
File: src/services/UserService.ts
Language: typescript
Node type: Service
Outline:
  class UserService (10-95)
    constructor (12-18)
    getUser (20-42)
    saveUser (44-70)
Imports:
  UserRepository from ./UserRepository
Used by:
  UserController.ts
Target excerpts:
  getUser (20-42)
```

El código fuente completo debe quedar como fallback o como expansión dirigida. Esto también abre la puerta a comandos IA más precisos como “explicar esta función”, “resumir esta clase”, “mostrar dependencias de este método” o “buscar dónde se define este símbolo”.

## UI propuesta

### Opción A: outline dentro del nodo

Bueno para pocos símbolos.

```text
┌──────────────────────────┐
│ Service                  │
│ UserService.ts           │
│ 4 symbols                │
├──────────────────────────┤
│ ▾ class UserService      │
│   • constructor          │
│   • getUser              │
│   • saveUser             │
└──────────────────────────┘
```

Riesgo: nodos muy grandes pueden ensuciar el grafo.

### Opción B: panel lateral de outline

Recomendado para primera versión.

```text
Click en nodo UserService.ts
→ panel lateral muestra outline completo
→ el nodo sigue compacto en el grafo
```

Ventajas:

- mantiene el grafo limpio;
- permite jerarquías largas;
- puede sumar búsqueda y navegación a línea;
- se parece más a VS Code.

### Opción C: expandir bajo demanda

Un híbrido: nodo compacto por defecto, botón para expandir resumen de símbolos.

## Plan de implementación por fases

### Fase 1 — Contratos y persistencia

- [ ] Crear `OutlineItem` en Rust y TypeScript.
- [ ] Crear tabla `outline_items` o persistir como JSON por archivo.
- [ ] Agregar `get_node_outline(file_id)`.
- [ ] Mantener UI sin cambios grandes.

### Fase 2 — Parser abstraction

- [ ] Crear `LanguageParser`.
- [ ] Crear `ParserRegistry`.
- [ ] Migrar TypeScript al nuevo contrato.
- [ ] Migrar Rust al nuevo contrato.
- [ ] Implementar recorrido recursivo del AST para capturar jerarquía (`class → method`, `module → function`, `interface → property`).
- [ ] Dejar fallback para extensiones desconocidas.

### Fase 3 — UI Outline

- [ ] Agregar panel lateral de outline al seleccionar nodo.
- [ ] Mostrar icono/tipo/nombre/rango de línea.
- [ ] Permitir colapsar/expandir hijos.
- [ ] Mostrar estado vacío: `No symbols detected`.

### Fase 4 — Mejor clasificación, métricas y contexto IA

- [ ] Usar outline para mejorar `NodeType`.
- [ ] Mostrar conteo real de símbolos.
- [ ] Detectar archivos con parser no soportado.
- [ ] Agregar métricas por archivo: clases, funciones, exports, imports.
- [ ] Modificar `engine/src/ai/context.rs` para construir contexto desde outline/imports/dependencias.
- [ ] Agregar extracción dirigida por rango para símbolos relevantes.
- [ ] Mantener lectura completa/truncada como fallback, no como camino principal.

### Fase 5 — Navegación semántica para IA

- [ ] Agregar búsqueda de símbolos por nombre/tipo.
- [ ] Agregar API para obtener fuente de un `OutlineItem` por rango.
- [ ] Permitir preguntas IA enfocadas en símbolo: clase, función, método o módulo.
- [ ] Usar outline para proponer qué archivos/símbolos leer antes de expandir contexto.

### Fase 6 — Nuevos lenguajes

Orden sugerido:

1. TypeScript / TSX
2. JavaScript / JSX
3. Rust
4. Python
5. Go

## Riesgos y decisiones abiertas

| Tema                    | Riesgo                                                        | Decisión sugerida                                            |
| ----------------------- | ------------------------------------------------------------- | ------------------------------------------------------------ |
| Outline dentro del nodo | Puede saturar el grafo                                        | Empezar con panel lateral                                    |
| Persistencia            | Tabla jerárquica puede complicarse                            | Empezar con JSON por archivo o tabla simple con `parent_id`  |
| Muchos lenguajes        | Mucha lógica duplicada                                        | Parser registry + trait común                                |
| Símbolos incompletos    | Tree-sitter queries varían por lenguaje                       | Tests fixture por lenguaje                                   |
| Performance             | Parsear símbolos/imports/outline por separado duplica trabajo | Parser debería producir un resultado agregado                |
| Contexto IA             | Truncar archivos grandes pierde estructura importante         | Usar outline + extractos dirigidos antes que archivo entero  |
| Índice semántico        | Diseñarlo solo para UI limita su valor                        | Tratar `OutlineItem` como base para navegación y contexto IA |

## Resultado esperado

Después de esta mejora, CodeAtlas debería permitir:

- entender qué contiene cada archivo sin abrirlo;
- navegar símbolos tipo VS Code;
- mejorar la clasificación de nodos;
- sumar lenguajes de forma ordenada;
- construir análisis arquitectónico sobre estructura real, no solo sobre nombres de archivo;
- darle a la IA una forma barata y precisa de navegar código: primero estructura, después rangos puntuales.

## Próximo paso recomendado

Crear un cambio SDD pequeño para `outline-parser-abstraction` con alcance limitado:

1. contratos `OutlineItem`;
2. parser registry;
3. outline para TypeScript/Rust;
4. comando `get_node_outline`;
5. panel lateral básico;
6. primer contexto IA basado en outline para reemplazar truncado bruto como camino principal.
