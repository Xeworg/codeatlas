# Explore — outline-parser-abstraction

## Resumen ejecutivo

Este cambio propone convertir el uso actual de Tree-sitter en CodeAtlas en una capa de análisis semántico reutilizable. El resultado visible inicial será un outline tipo VS Code por archivo/nodo, pero la decisión arquitectónica principal es más amplia: usar ese outline como índice semántico para que la IA pueda navegar estructura, imports, dependencias y rangos de símbolos antes de leer código fuente completo.

## Problema

CodeAtlas ya tiene grafo de archivos y extracción básica de símbolos/imports, pero la información sigue siendo demasiado plana para tres casos importantes:

1. UI: los nodos muestran poco detalle interno y pueden aparecer como `Unknown` o `0 symbols`.
2. Arquitectura: la clasificación depende demasiado de nombres/rutas y no de estructura real.
3. IA: el contexto puede depender de leer archivos completos o truncados por bytes, perdiendo estructura en archivos grandes.

Tree-sitter ya está disponible en el backend, por lo que conviene usarlo como fuente determinista para estructura de código antes de pedir inferencia a una IA.

## Contexto técnico observado

- `engine/src/scanner/parser.rs` contiene el parser monolítico actual (`CodeParser::parse_file`) con dispatch por extensión y extracción plana.
- `engine/src/models/file.rs` contiene `SymbolInfo`, `ImportInfo` y `SymbolKind`; falta un modelo jerárquico `OutlineItem`.
- `engine/src/db/schema.rs` y migraciones actuales no tienen almacenamiento de outline.
- `src-tauri/src/commands.rs` contiene `scan_project`, `get_graph`, `get_node_details` y comandos IA; falta `get_node_outline`.
- `engine/src/ai/context.rs` es el punto principal para cambiar contexto IA desde fuente truncada hacia outline/imports/dependencias/extractos dirigidos.
- `src/components/panel/DetailPanel.tsx` y `src/components/panel/SymbolList.tsx` son los puntos más seguros para introducir UI de outline sin ensuciar el grafo.
- `src/components/graph/GraphNodeComponent.tsx` debería quedar compacto en la primera entrega.

## Alcance explorado

### In scope

- Crear modelo `OutlineItem`/`OutlineItemKind` en Rust y TypeScript.
- Crear `ParseResult` para producir símbolos, imports y outline desde una pasada semántica.
- Introducir `LanguageParser` y `ParserRegistry` para separar parsers por lenguaje.
- Migrar TypeScript/TSX y Rust como primeros lenguajes soportados.
- Agregar persistencia inicial de outline por archivo, preferentemente JSON por `file_id`.
- Agregar comando Tauri `get_node_outline(file_id)` y wrapper frontend.
- Agregar `OutlineView` en panel lateral.
- Empezar integración IA usando outline como contexto primario y fuente completa/truncada como fallback.

### Out of scope inicial

- Soportar todos los lenguajes desde el primer cambio.
- Reemplazar el grafo actual.
- Agregar edición de código desde outline.
- Construir búsquedas globales avanzadas de símbolos en la primera versión.
- Depender de IA para extraer estructura que Tree-sitter puede extraer de forma determinista.
- Implementar análisis semántico perfecto por método/flujo de control.

## Decisiones iniciales recomendadas

| Tema                 | Recomendación                                                                                             |
| -------------------- | --------------------------------------------------------------------------------------------------------- |
| Contrato parser      | Preferir `parse_all() -> ParseResult` sobre tres métodos públicos separados.                              |
| Persistencia         | Empezar con tabla `outline_items(file_id, outline_json, generated_at)`.                                   |
| UI inicial           | Panel lateral en `DetailPanel`, no outline completo dentro del nodo.                                      |
| IA                   | Usar outline/imports/dependencias como contexto principal y leer código por rango solo cuando haga falta. |
| Lenguajes iniciales  | TypeScript/TSX y Rust; JS/JSX puede seguir por cercanía con TypeScript.                                   |
| Normalización futura | Dejar tabla relacional por símbolo para una fase posterior si hace falta búsqueda global eficiente.       |

## Riesgos

1. **Scope creep:** mezclar parser abstraction, persistencia, UI e IA puede superar el presupuesto de revisión.
2. **Diferencias de gramática:** Tree-sitter TypeScript/Rust no exponen exactamente los mismos nodos; se necesitan fixtures por lenguaje.
3. **Jerarquía incompleta:** capturar `class -> method`, `impl -> method`, `module -> function`, `interface -> field` exige recorrido recursivo cuidadoso.
4. **Contratos UI/BE:** `OutlineItemKind` no debe acoplarse accidentalmente a `SymbolKind` si tienen objetivos distintos.
5. **Performance:** parsear varias veces el mismo archivo puede ser barato al inicio pero caro en proyectos medianos; `ParseResult` reduce ese riesgo.
6. **IA con demasiado contexto:** el índice semántico puede volverse otro payload grande si no hay modos (`summary`, `focused`, `full`).

## Supuestos

- SQLite local sigue siendo la fuente de persistencia principal.
- Tree-sitter seguirá siendo la fuente determinista para estructura de código.
- La primera UI de outline puede convivir con `SymbolList` o reemplazarla gradualmente.
- El proyecto acepta migraciones aditivas; la próxima migración disponible sería `007_outline_items.sql`.
- Strict TDD está activo para implementación posterior; este explore no ejecuta cambios de código productivo.
- El presupuesto de revisión de esta sesión es 600 líneas antes de recomendar partir el trabajo.

## Preguntas abiertas para proposal/spec

1. ¿`OutlineItem.id` debe ser estable entre scans usando path + rango + nombre, o UUID nuevo por scan?
2. ¿`OutlineItemKind` debe mapear exactamente a `SymbolKind` o mantener un contrato separado para UI/IA?
3. ¿Persistimos outline siempre durante `scan_project` o permitimos reparse bajo demanda para archivos no escaneados?
4. ¿La primera integración IA debe modificar `explain_node` solamente o también `chat`?
5. ¿JS/JSX entra en el mismo primer slice que TypeScript/TSX o queda como extensión inmediata posterior?
6. ¿La UI debe mantener `SymbolList` como fallback visible si el outline está vacío?

## Recomendación de slicing

Para evitar un PR demasiado grande, conviene planificar el cambio en slices:

1. **Backend semantic foundation:** modelos, `ParseResult`, registry, parsers TS/Rust y tests.
2. **Persistence/API:** migration `007`, queries, `scan_project`, `get_node_outline`, wrappers TS.
3. **UI outline:** `OutlineView` en panel lateral, estados vacíos y loading/error.
4. **AI context:** `engine/src/ai/context.rs` con summary/focused fallback y extractos por rango.

Si el diseño final estima más de 600 líneas cambiadas, el cambio debería dividirse en PRs encadenados.

## Próximo recomendado

Pasar a `proposal` para fijar alcance del cambio `outline-parser-abstraction`, criterios de aceptación y non-goals. La proposal debe dejar claro que el objetivo no es solo UI de outline, sino una base semántica Tree-sitter-first para navegación humana e IA.
