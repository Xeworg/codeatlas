# Proposal — outline-parser-abstraction

## Resumen

Este cambio formaliza una capa semántica Tree-sitter-first para CodeAtlas. El resultado visible será un outline tipo VS Code por nodo/archivo, pero el objetivo arquitectónico es más amplio: que el backend produzca una estructura navegable de símbolos, imports y rangos para que la UI y la IA puedan entender código sin depender de archivos completos o truncados.

## Intento / Objetivo

Crear la base `outline-parser-abstraction` para que CodeAtlas pueda:

1. extraer estructura jerárquica por archivo usando Tree-sitter;
2. separar parsers por lenguaje mediante contratos estables;
3. exponer outline de nodo/archivo a la UI;
4. persistir o recuperar outline de forma eficiente;
5. construir contexto IA desde estructura semántica antes de leer código fuente completo.

La mejora debe tratar `OutlineItem` como modelo de dominio compartido, no solo como componente visual.

## Alcance (In Scope)

### A) Contratos semánticos backend

- Crear `OutlineItem` y `OutlineItemKind` en Rust.
- Crear equivalente TypeScript en `src/lib/types.ts`.
- Introducir `ParseResult` con `symbols`, `imports` y `outline`.
- Introducir trait `LanguageParser` y `ParserRegistry`.
- Migrar el flujo inicial de parsing para TypeScript/TSX y Rust.
- Mantener fallback seguro para extensiones no soportadas.

### B) Persistencia y API

- Agregar migración aditiva para `outline_items`.
- Persistir outline por archivo, preferentemente como JSON por `file_id`.
- Agregar queries para guardar y recuperar outline.
- Integrar outline al flujo `scan_project` sin romper símbolos/imports actuales.
- Agregar comando Tauri `get_node_outline(file_id)`.
- Agregar wrapper frontend `getNodeOutline(fileId)`.

### C) UI de outline

- Agregar una vista jerárquica `OutlineView`.
- Integrar outline en el panel lateral de detalles al seleccionar un nodo.
- Mostrar tipo, nombre y rango de línea por símbolo.
- Soportar estados loading/error/empty.
- Mantener el nodo del grafo compacto en la primera versión.

### D) Contexto IA semántico

- Modificar la construcción de contexto IA para priorizar outline, imports, dependencias y rangos relevantes.
- Mantener lectura completa/truncada como fallback, no como camino principal.
- Preparar modos conceptuales de contexto: `summary`, `focused`, `full`.
- Permitir que futuras preguntas IA puedan expandir solo símbolos específicos.

## Non-Goals

- No implementar todos los lenguajes en este cambio.
- No reemplazar el grafo actual.
- No agregar editor de código o navegación IDE completa.
- No construir búsqueda global avanzada de símbolos en la primera entrega.
- No depender de IA para extraer símbolos que Tree-sitter puede extraer determinísticamente.
- No implementar análisis perfecto de flujo de control o dependencias a nivel método.
- No normalizar todos los outline items en tabla relacional salvo que el diseño lo justifique.

## Decisiones propuestas

| Tema                 | Decisión propuesta                                                                           |
| -------------------- | -------------------------------------------------------------------------------------------- |
| Parser contract      | Usar `parse_all() -> ParseResult` como contrato principal para evitar recorridos duplicados. |
| Persistencia inicial | Usar `outline_items(file_id, outline_json, generated_at)` como tabla simple por archivo.     |
| UI inicial           | Mostrar outline en `DetailPanel`; no expandirlo dentro de cada nodo del grafo.               |
| IA                   | Construir contexto con outline/imports/dependencias y leer código por rango bajo demanda.    |
| Primeros lenguajes   | TypeScript/TSX y Rust; JS/JSX puede incorporarse si cae naturalmente del parser TS/JS.       |
| Review workload      | Si el diseño supera 600 líneas estimadas, dividir en PRs encadenados.                        |

## Criterios de aceptación

1. `scan_project` sigue generando archivos, símbolos e imports como antes.
2. Para archivos TypeScript/TSX y Rust soportados, el backend puede generar outline jerárquico básico.
3. `get_node_outline(file_id)` devuelve un árbol `OutlineItem[]` estable y serializable.
4. La UI muestra el outline en el panel lateral al seleccionar un nodo.
5. Archivos sin parser soportado o sin símbolos muestran un estado vacío claro.
6. El contexto IA para explicación de nodo incluye resumen semántico basado en outline/imports/dependencias antes de usar fuente truncada.
7. La implementación incluye pruebas o fixtures que validen al menos jerarquía básica TypeScript y Rust.
8. Las migraciones son aditivas y no rompen bases existentes.
9. El diseño mantiene `OutlineItemKind` separado de `SymbolKind` salvo mapeo explícito.
10. Si el cambio excede el presupuesto de revisión, queda dividido en slices revisables.

## Riesgos clave

1. **Scope inflation:** parser abstraction + persistencia + UI + IA puede volverse grande.
2. **Diferencias Tree-sitter por lenguaje:** cada gramática requiere reglas específicas.
3. **Jerarquía incompleta:** métodos, impls, módulos e interfaces pueden omitirse si el recorrido no es recursivo.
4. **Contratos inestables:** acoplar `OutlineItemKind` a `SymbolKind` puede limitar UI/IA.
5. **Performance:** parsear varias veces el mismo archivo puede duplicar costo.
6. **Contexto IA excesivo:** un outline grande también puede necesitar compactación.

## Mitigación inicial

- Diseñar primero contratos y fixtures.
- Usar `ParseResult` para compartir una pasada de parsing.
- Persistir JSON por archivo para evitar complejidad relacional temprana.
- Mantener UI en panel lateral para no saturar el grafo.
- Introducir contexto IA semántico en modo incremental y con fallback.
- Dividir implementación en slices si la estimación supera 600 líneas.

## Rollback

Si el cambio afecta estabilidad de scan o graph:

1. desactivar consumo de outline en UI/IA;
2. mantener `get_node_details` y símbolos/imports existentes como fallback;
3. revertir integración `scan_project` con outline si rompe persistencia;
4. conservar migración aditiva sin lectura activa, o revertir el slice completo antes de release;
5. restaurar contexto IA anterior basado en fuente truncada si la salida semántica degrada calidad.

## Áreas afectadas

- `engine/src/scanner/parser.rs`
- `engine/src/scanner/parser/*` nuevo módulo propuesto
- `engine/src/models/file.rs`
- `engine/src/models/mod.rs`
- `engine/src/db/schema.rs`
- `engine/src/db/migrations.rs`
- `engine/migrations/007_outline_items.sql`
- `engine/src/db/queries.rs`
- `engine/src/ai/context.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src/lib/types.ts`
- `src/lib/tauri-api.ts`
- `src/components/panel/DetailPanel.tsx`
- `src/components/panel/OutlineView.tsx`
- `src/components/panel/SymbolList.tsx`
- `src/components/graph/GraphNodeComponent.tsx` solo si se mejora resumen compacto

## Referencias de evidencia

- `docs/PLAN_OUTLINE_TREE_SITTER_Y_PARSERS.md`
- `openspec/changes/outline-parser-abstraction/explore.md`
- `openspec/config.yaml`
- `engine/src/scanner/parser.rs`
- `engine/src/ai/context.rs`
- `src-tauri/src/commands.rs`
- `src/components/panel/DetailPanel.tsx`

## Próximo paso

Pasar a `spec` para convertir esta propuesta en requisitos verificables con escenarios Given/When/Then, especialmente para:

1. extracción jerárquica TypeScript/Rust;
2. persistencia y API de outline;
3. renderizado de panel lateral;
4. construcción de contexto IA semántico.
