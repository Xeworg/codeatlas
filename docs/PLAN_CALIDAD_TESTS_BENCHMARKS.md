# CodeAtlas — Plan de Calidad: Tests, Benchmarks y Performance

**Versión:** pre-SDD v1
**Alcance:** estrategia de testing y calidad para v1, con extensión a v2/v3

---

## 1. Pirámide de testing

```
         ╱ E2E ╲          ← Tauri + UI integrados
        ╱─────────╲
       ╱Integration╲       ← UI-BE contracts, DB queries
      ╱───────────────╲
     ╱   Unit Tests    ╲    ← Rust functions, React components
    ╱─────────────────────╲
```

### 1.1 Distribución objetivo (v1)

| Nivel       | % del total | Framework                                             | Ejecución |
| ----------- | ----------- | ----------------------------------------------------- | --------- |
| Unit        | 60%         | `cargo test` + `vitest`                               | < 30s     |
| Integration | 30%         | `cargo test` (DB + fixtures) + `vitest` (store hooks) | < 60s     |
| E2E         | 10%         | Tauri + Playwright-like                               | < 3 min   |

---

## 2. Tests unitarios

### 2.1 Backend (Rust)

**Framework:** `cargo test` con `#[cfg(test)]`

**Cobertura requerida:**
| Módulo | Cobertura mínima | Tipo de tests |
|---|---|---|
| `scanner::walker` | 90% | Path filtering, exclusiones, symlinks |
| `scanner::parser` | 85% | Tree-sitter extracción, edge cases TS/JSX |
| `graph::builder` | 90% | Construcción de grafo, aliases tsconfig |
| `graph::resolver` | 85% | Path resolution, módulos externos |
| `ai::context` | 80% | Construcción de contexto, truncado |
| `ai::provider` | 75% | Config, errores de conexión |
| `db::queries` | 90% | CRUD, constraints, cascades |

**Ejemplo test unitario:**

```rust
#[test]
fn walker_ignores_node_modules() {
    let tmp = tempdir().unwrap();
    create_file(tmp.path().join("node_modules/lib.js"));
    create_file(tmp.path().join("src/index.ts"));
    let files = walk_dir(tmp.path(), &default_excludes());
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("index.ts"));
}
```

### 2.2 Frontend (TypeScript/React)

**Framework:** `vitest` + `@testing-library/react`

**Cobertura requerida:**
| Capa | Cobertura mínima | Enfoque |
|---|---|---|
| `stores/*` | 85% | Estados, acciones, selectores Zustand |
| `hooks/*` | 80% | Lógica de suscripción a stores |
| `lib/graph-layout.ts` | 90% | Dagre/ELK, posiciones |
| `lib/tauri-api.ts` | 90% | Tipos exportados (compilación) |

---

## 3. Tests de integración

### 3.1 Backend integration (Rust + SQLite)

**Framework:** `cargo test -- --ignored` (requieren DB)

| Test suite          | Qué cubre                                                   |
| ------------------- | ----------------------------------------------------------- |
| `db_integration`    | Insertar proyecto, recuperar archivos, constraints FK       |
| `scan_integration`  | Escanear fixture → parsear → guardar en DB → recuperar      |
| `graph_integration` | Construir grafo desde fixture, resolver aliases, serializar |
| `ai_integration`    | (con mock HTTP) Construir request, parsear response         |

### 3.2 Frontend integration

| Test suite                   | Qué cubre                                          |
| ---------------------------- | -------------------------------------------------- |
| `graphStore` + mock `invoke` | Ciclo completo: recibir GraphData → nodos visibles |
| `chatStore`                  | Historial en memoria, append, limpiar              |
| `projectStore`               | Estados scan: idle → scanning → ready → error      |

### 3.3 Contratos (contract tests)

**Ubicación:** `tests/contracts/v1/`
**Formato:** JSON snapshots comparados automáticamente.

```rust
#[test]
fn contract_scan_project_snapshot() {
    let result = scan_project_fixture("simple-ts").unwrap();
    let json = serde_json::to_string_pretty(&result).unwrap();
    insta::assert_snapshot!("scan_project", json);
}
```

> `insta` o crate equivalente para snapshot testing.

---

## 4. Tests E2E

### 4.1 Herramientas

- **v1:** testing manual con checklist E2E. Automatización con Tauri + WebDriver si el tiempo lo permite.
- **v2:** Tauri WebDriver oficial o Playwright con Tauri fixture.
- **v3:** Suite completa E2E por flujo de usuario.

### 4.2 Checklist E2E mínimo v1

| #   | Flujo            | Criterio                                           |
| --- | ---------------- | -------------------------------------------------- |
| E1  | Abrir proyecto   | Diálogo nativo → escaneo inicia → progreso visible |
| E2  | Ver grafo        | Nodos/aristas renderizados tras escaneo            |
| E3  | Navegar grafo    | Zoom, pan, clic en nodo cambia selección           |
| E4  | Ver detalles     | Panel inferior muestra metadata del nodo           |
| E5  | Explicación IA   | Click en "Explicar" → respuesta en <5s             |
| E6  | Chat IA          | Pregunta → respuesta con referencias               |
| E7  | Buscar nodo      | Búsqueda enfoca nodo en grafo                      |
| E8  | Cambiar proyecto | Reset estado → nuevo escaneo                       |
| E9  | Sin API key      | Placeholder visible, sin error fatal               |
| E10 | Error de scan    | Mensaje claro, posibilidad de reintentar           |

---

## 5. Performance benchmarks

### 5.1 Escenarios de benchmark

| Escenario            | Archivos        | Métrica                   | Objetivo v1       |
| -------------------- | --------------- | ------------------------- | ----------------- |
| `small-project`      | 100             | Scan time                 | < 500ms           |
| `medium-project`     | 1000            | Scan time                 | < 3s              |
| `large-project`      | 5000            | Scan time                 | < 10s             |
| `graph-render-large` | 1000 nodos      | FPS / interaction latency | 30+ FPS / < 100ms |
| `ai-explain`         | Archivo 200 LOC | Respuesta total           | < 5s              |
| `memory-medium`      | 1000 archivos   | RSS                       | < 300 MB          |

### 5.2 Herramientas

- **Rust:** `criterion` para micro-benchmarks (scan, parse, graph build).
- **Frontend:** React Profiler + `performance.now()` wrappers.
- **CI:** GitHub Actions con hardware consistente (ubuntu-latest, 2-core). Benchmarks como informational (no blocking), pero con alerta si degradan >20%.

### 5.3 Benchmark suite

```
engine/benches/
├── scan_benchmark.rs
├── parse_benchmark.rs
├── graph_build_benchmark.rs
└── fixtures/
    ├── small/   (100 files)
    ├── medium/  (1000 files)
    └── large/   (5000 files, generados proceduralmente)
```

---

## 6. Métricas de calidad (v1 + v2 resultados reales)

| Métrica                     | Objetivo         | Resultado real v2                                             | Estado |
| --------------------------- | ---------------- | ------------------------------------------------------------- | ------ |
| Cobertura unitaria Rust     | ≥ 80%            | Tests passing (55 tests)                                      | ✅     |
| Cobertura unitaria TS       | ≥ 75%            | Tests passing (57 tests)                                      | ✅     |
| Regresiones                 | 0 en CI          | CI limpio (cargo test + npm test + clippy + lint + typecheck) | ✅     |
| Performance scan            | < 10s (5k files) | Benchmark scaffold only — sin mediciones reales               | ⚠️     |
| Bugs P0 abiertos al release | 0                | Ningún P0 abierto                                             | ✅     |

**Nota:** Las métricas de cobertura (tarpaulin, vitest coverage) no se reportan formalmente aún. El benchmark de performance scan es scaffold: existe el archivo de benchmark pero no hay fixture real de 1000+ archivos ni mediciones validadas.

### v2 Test Summary

| Suite                     | Tests                       | Status       |
| ------------------------- | --------------------------- | ------------ |
| Rust (`cargo test --lib`) | 55 tests (valor verificado) | ✅ GREEN     |
| TS unit (v1+v2)           | 57 tests                    | ✅ GREEN     |
| **Total verificado**      | **112 tests**               | ✅ ALL GREEN |

> Nota: El desglose fino por módulo Rust quedó pendiente de reconciliar con `cargo test -- --list`.

---

## 7. CI pipeline (GitHub Actions)

```yaml
jobs:
  test-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --all-features
      - run: cargo test -- --ignored # integration
      - run: cargo tarpaulin --out Xml

  test-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: npm run lint
      - run: npm run test -- --coverage

  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - run: cargo bench
      - run: node scripts/check-bench-regression.js

  contracts:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test -- contracts
```

---

## 8. Extensión v2/v3

| Añadido                                                   | v2  | v3  |
| --------------------------------------------------------- | --- | --- |
| Integration tests para arquitectura detectada             | ✅  | ✅  |
| Integration tests para colaboración (snapshot round-trip) | —   | ✅  |
| E2E automatizados con WebDriver                           | ✅  | ✅  |
| Benchmarks de impacto de cambio                           | ✅  | ✅  |
| Health metric validation tests                            | —   | ✅  |
| Performance budget por feature                            | ✅  | ✅  |

---

## 9. Supuestos no resueltos

| #   | Supuesto                                                                | Dueño     | Target      |
| --- | ----------------------------------------------------------------------- | --------- | ----------- |
| Q1  | Generación procedural de fixtures grandes es suficiente para benchmarks | Backend   | Sprint 1    |
| Q2  | Tauri WebDriver estará estable para E2E en v2                           | Tech Lead | v2 planning |
| Q3  | `cargo-tarpaulin` funciona con workspace de Tauri + engine crate        | Backend   | Sprint 0    |

---

_Documento pre-SDD. Actualizar con métricas reales post-implementación._
