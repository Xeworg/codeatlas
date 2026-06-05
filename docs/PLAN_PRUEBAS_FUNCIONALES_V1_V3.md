# CodeAtlas — Plan de pruebas funcionales (v1→v3)

## Objetivo

Validar de punta a punta todas las capacidades implementadas en v1, v2 y v3, incluyendo app nativa Tauri, backend Rust, frontend React, contratos y degradación controlada.

---

## 1) Precondiciones

## Entorno

- Fedora con dependencias Tauri instaladas.
- `PKG_CONFIG_PATH` disponible en shell (`~/.zshrc` / `~/.zprofile`).
- Node, npm, Rust, cargo instalados.

## Arranque

```bash
source ~/.zshrc
npx tauri dev
```

## Checks base (antes de pruebas manuales)

```bash
cargo check --manifest-path src-tauri/Cargo.toml
npm run typecheck
npm run lint
npm run test
```

---

## 2) Pruebas de arranque y plataforma

### P-01: App nativa abre

1. Ejecutar `npx tauri dev`.
2. Verificar que se abre ventana nativa CodeAtlas.

**Esperado:** ventana visible, sin error de `dialog.open not allowed`, sin crash al iniciar.

### P-02: Selector de proyecto

1. Click en **Abrir proyecto**.
2. Seleccionar carpeta de proyecto válida.

**Esperado:** dialog abre correctamente y no muestra errores de permisos Tauri.

---

## 3) Pruebas v1 (base)

### P-10: Escaneo de proyecto

1. Abrir un proyecto TS/JS/Rust.
2. Ejecutar escaneo.

**Esperado:** estado pasa a ready, contador de archivos/símbolos/imports > 0.

### P-11: Grafo y navegación

1. Ver grafo.
2. Buscar nodo por nombre.
3. Abrir detalles de nodo.

**Esperado:** nodos/aristas renderizados, búsqueda funcional, detalles coherentes.

### P-12: IA base

1. Configurar AI.
2. Ejecutar `explain_node` y chat.

**Esperado:** respuesta válida sin exponer API key en UI/estado.

---

## 4) Pruebas v2 (análisis avanzado)

### P-20: Detección de arquitectura

1. Pedir arquitectura detectada para el proyecto.

**Esperado:** `{ pattern, confidence, evidence }` con contrato válido.

### P-21: Impact analysis

1. Seleccionar nodo.
2. Ejecutar análisis de impacto.

**Esperado:** `affectedNodes` + `impactScore` + explicación.

### P-22: Graph insights

1. Ejecutar insights.

**Esperado:** ciclos/hotspots/acoplamiento/densidad disponibles o fallback degradado.

### P-23: Export JSON

1. Exportar vista a JSON.

**Esperado:** payload `ExportPayload` válido.

### P-24: i18n base

1. Recorrer UI principal.

**Esperado:** textos mediante catálogo `locales/es.json` sin hardcode inconsistente.

---

## 5) Pruebas v3 H1 (hardening heredado)

### P-30: Wiring App-level T5.6

1. Verificar que en flujo principal aparecen:
   - `AnalyticsViewSelector`
   - `ArchitectureCard`
   - `ImpactPanel`
   - `InsightsPanel`

**Esperado:** componentes operativos, sin wiring roto.

### P-31: Degraded-mode 8/8

Validar escenarios:

1. PNG fallback via mock.
2. Contract mismatch → update required.
3. AI no configurada.
4. AI timeout.
   5-8. Escenarios backend ya cubiertos.

**Esperado:** fallback correcto, UI estable, sin crash.

### P-32: Benchmarks fixture + evidencia

1. Ejecutar harness de benchmark/documentación de benchmark.
2. Verificar fixture real (1200 archivos).

**Esperado:** evidencia NFR registrada en `tests/benchmarks/benchmarks.md`.

---

## 6) Pruebas v3 H2 (colaboración base)

### P-40: Workspaces

1. Crear workspace.
2. Listar workspaces.
3. Asociar proyecto a workspace.
4. Listar proyectos de workspace.

**Esperado:** persistencia correcta y respuestas con contratos v3.

### P-41: Snapshots

1. Crear snapshot con label.
2. Listar snapshots (con filtro workspace si aplica).
3. Obtener snapshot por id.

**Esperado:** `payload_json` persistido (graph + insights + arch_detection).

### P-42: Annotations

1. Agregar comentario (nodeId, author, text, kind).
2. Listar comentarios por proyecto/nodo.

**Esperado:** creación/listado consistente; orden temporal correcto.

---

## 7) Pruebas v3 H3 (ejecutivo)

### P-50: Health timeline

1. Consultar timeline por rango `from/to`.

**Esperado:** orden ascendente por `recorded_at`, contrato válido.

### P-51: Executive summary

1. Solicitar resumen ejecutivo por workspace.

**Esperado:** resumen coherente con datos del workspace (proyectos, hotspots, tendencia).

### P-52: Snapshot diff

1. Comparar snapshot base vs target.

**Esperado:** diff estructurado y estable.

### P-53: C4 view

1. Solicitar vista C4 nivel 1 y 2.

**Esperado:** respuesta con shape esperado y warnings controlados si faltan datos.

---

## 8) Pruebas de regresión rápida (smoke)

Ejecutar después de cambios relevantes:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run test
npm run typecheck
npm run lint
```

**Esperado:** verde, salvo tests Tauri-invoke marcados como expected-RED fuera de runtime.

---

## 9) Criterio de salida

Se considera PASS cuando:

- App nativa abre y permite abrir proyecto.
- Flujo v1/v2/v3 ejecuta sin bloqueos.
- Contratos v3 clave responden correctamente.
- Degraded-mode y fallback funcionan.
- No hay regresiones críticas en checks automáticos.

---

## 10) Evidencia a guardar por corrida

- Fecha/hora
- Commit probado
- Entorno (OS, Node, Rust)
- Resultado por caso (PASS/FAIL)
- Logs relevantes (`/tmp/codeatlas-tauri.log`)
- Capturas de errores UI
