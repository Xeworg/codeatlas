# Delta Spec: multi-language-dispatch

## Purpose

Ruta única de dispatch a través de `ParserRegistry`. Cierra la divergencia entre `CodeParser::parse_file` (heredada) y `CodeParser::parse_file_all` (registry), dejando `parse_file` como shim deprecado. Define el contrato que futuros parsers (Python, Go, Java) deben respetar y que `src-tauri/src/commands.rs` invoca una sola vez por archivo.

## Requirements

### Requirement: ParserRegistry es el Único Punto de Dispatch

`ParserRegistry` MUST ser la única superficie de dispatch usada por Tauri commands y por la lógica de `engine/`. `CodeParser` MUST delegar al registry para parsear archivos en v1. El sistema MUST NOT mantener lógica de extracción de symbols/imports paralela a la del registry.

#### Scenario: Registry despacha por extensión

- GIVEN un `ParserRegistry` con `TypeScriptParser` y `RustParser` registrados
- WHEN se invoca `registry.parse_file_all("foo.ts", content, ".ts")` o `"foo.rs"` con `".rs"`
- THEN el registry MUST despachar al parser correspondiente
- AND el `ParseResult` retornado MUST ser idéntico al del parser invocado directamente

#### Scenario: Extensión desconocida no panica

- GIVEN un archivo con extensión no registrada (ej. `.xyz`)
- WHEN se invoca `registry.parse_file_all(...)`
- THEN el sistema MUST retornar un `Result` con error tipado
- AND el sistema MUST NOT panicar

### Requirement: Shim Deprecated para `CodeParser::parse_file`

`CodeParser::parse_file` MUST sobrevivir como shim marcado `#[deprecated(note = "use ParserRegistry::parse_file_all instead")]`. El shim MUST delegar al registry y MUST producir el mismo `ParseResult` observable (mismos `symbols`, `imports`, `outline`, `lexical_kind`, `references`) que la ruta registry directa. La remoción del shim queda para un follow-up change.

#### Scenario: Shim produce misma salida que registry

- GIVEN un fixture TS arbitrario
- WHEN se invoca `CodeParser::parse_file(path, content, ext)` y `ParserRegistry::parse_file_all(...)` por separado
- THEN ambos retornos MUST ser equivalentes en `symbols`, `imports`, `outline`, `lexical_kind` y `references`
- AND el compilador MUST emitir warning de deprecation al usar `parse_file`

#### Scenario: Deprecation note dirige a registry

- GIVEN el código fuente de `code_parser.rs`
- WHEN un desarrollador compila
- THEN el atributo `#[deprecated]` MUST contener un `note` que mencione `ParserRegistry::parse_file_all`

### Requirement: scan_project usa Registry Una Sola Vez

`src-tauri/src/commands.rs::scan_project` MUST invocar el registry una sola vez por archivo y derivar `symbols`, `imports`, `outline`, `references` y `lexical_kind` del mismo `ParseResult`. El sistema MUST NOT invocar `parse_file` y `parse_file_all` por separado para el mismo archivo.

#### Scenario: scan_project hace una sola llamada por archivo

- GIVEN un árbol de proyecto con N archivos
- WHEN se ejecuta `scan_project`
- THEN el sistema MUST invocar el registry exactamente N veces
- AND el log de scan completion MUST evidenciar el conteo o un `duration_ms` consistente con single-pass

#### Scenario: Derivación es local al ParseResult

- GIVEN un `ParseResult` con todos los campos poblados
- WHEN `scan_project` persiste resultados
- THEN el comando MUST usar `result.symbols`, `result.imports`, `result.outline`, `result.references`, `result.lexical_kind` del mismo struct
- AND MUST NOT invocar `parse_file` ni `parse_file_all` adicionales

### Requirement: get_node_outline usa Registry Una Sola Vez

`src-tauri/src/commands.rs::get_node_outline` MUST invocar el registry una sola vez y derivar el outline completo del `ParseResult.outline` retornado. El sistema MUST NOT llamar `CodeParser::parse_file` seguido de `parse_file_all` (o viceversa) para construir el outline.

#### Scenario: get_node_outline es single-call

- GIVEN un `file_id` y su contenido
- WHEN se ejecuta `get_node_outline`
- THEN el comando MUST invocar el registry exactamente una vez
- AND el outline retornado MUST provenir del `ParseResult.outline` único

### Requirement: Add-a-Language no Toca Dispatch

Incorporar un nuevo parser (cuarto lenguaje) MUST requerir SOLO `impl LanguageParser` y `registry.register(...)`. El sistema MUST NOT requerir cambios a `CodeParser`, `commands.rs` ni a la lógica de selección por extensión. El registry MUST resolver el nuevo lenguaje por sus `extensions` declaradas en el trait.

#### Scenario: Registro de stub no toca dispatch

- GIVEN un `PythonParser` que implementa `LanguageParser` con `extensions = [".py"]`
- WHEN se ejecuta `registry.register(Arc::new(PythonParser))` en el setup de la app
- THEN archivos `.py` MUST despacharse a `PythonParser`
- AND `commands.rs::scan_project` MUST procesarlos sin código específico por lenguaje

#### Scenario: Shim sigue funcionando con nuevo parser

- GIVEN el registro ampliado con un cuarto parser
- WHEN un consumidor legacy invoca `CodeParser::parse_file("foo.py", content, ".py")`
- THEN el shim MUST delegar al registry
- AND MUST retornar el `ParseResult` del `PythonParser`

## Out of Scope

- Remoción del shim `parse_file`: queda para un follow-up change.
- Resolución cross-file, type inference y control-flow.
- Persistencia de los nuevos campos en SQLite.
- Cambios al frontend o prompts IA.
- Implementación real de parsers Python, Go o Java: el contrato verifica que su adición futura sea trivial, pero los stubs no son parte de este cambio.
- Performance NFR más allá de mantener el path registry dentro del mismo orden de magnitud que el heredado.
