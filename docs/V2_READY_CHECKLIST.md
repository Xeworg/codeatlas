# CodeAtlas v2 — Readiness Checklist

Aligned with: `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md`, `docs/ARQUITECTURA_DATOS_V2_V3.md`, `docs/RIESGOS_DECISIONES_ABIERTAS.md`, `docs/PLAN_RELEASES_V1_V3.md`, `openspec/README.md`.

---

## 1. Scope Gate for v2

**In scope v2** (from plan maestro):

- Detección de arquitectura (MVC/Layered/Clean/Hexagonal) con score de confianza
- Aristas avanzadas (usage/calls) progresivas
- Motor de impacto de cambios
- Cálculo de ciclos y métricas de acoplamiento
- Endpoint de exportes (JSON/PNG)
- Filtros persistentes y agrupaciones
- Vista de ciclos y hotspots
- Tarjeta de arquitectura detectada con evidencia
- Tipo de diagrama adicional: Flujo de aplicación (beta)

**Out of scope v2** (no creeping):

- [ ] Multi-proyecto / workspace (v3)
- [ ] Snapshots colaborativos (v3)
- [ ] Dashboard ejecutivo con health timeline (v3)
- [ ] Comentarios/anotaciones en nodos (v3)
- [ ] Virtualización de nodos para >1000 nodos (v2 can defer)

**Approved v2 addition (controlled scope): i18n foundation only**

- [ ] Extract UI copy to `locales/es.json`
- [ ] Add `t('key')` helper and migrate current UI strings to keys
- [ ] Keep runtime language fixed to Spanish (no language switcher in this slice)
- [ ] Leave structure ready for future `locales/en.json` without component rewrites

**Scope decision gate**: Any proposed feature not in the "In scope" list above must be explicitly approved by product owner before entering v2 planning.

---

## 2. Required SDD Artifacts

- [ ] `openspec/changes/v2-advanced-analysis/proposal.md` — approved goals, in/out, metrics, non-goals, risks, rollback
- [ ] `openspec/changes/v2-advanced-analysis/spec.md` — canonical spec with acceptance criteria
- [ ] `openspec/changes/v2-advanced-analysis/design.md` — architecture, module boundaries, API contracts, error model
- [ ] `openspec/changes/v2-advanced-analysis/tasks.md` — executable tasks with dependencies and PR forecast
- [ ] `openspec/changes/v2-advanced-analysis/verify-report.md` — produced at SDD Verify
- [ ] `openspec/changes/v2-advanced-analysis/archive-report.md` — produced at SDD Archive
- [ ] `openspec/config.yaml` — updated for v2 change ID and phase
- [ ] `openspec/changes/archive/2026-06-01-v1-mvp-core/` — v1 archive must exist before v2 starts (from v1 SDD)

---

## 3. Contract Versioning Checklist

New contracts required for v2 (per `docs/PLAN_MAESTRO_SPRINTS_UI_BACKEND_V1_A_V3.md` v2 integrations and `docs/CHANGELOG_CONTRATOS.md` v2.0):

- [ ] `ArchitectureDetectionResult` — `{ pattern: string, confidence: number, evidence: object }`
- [ ] `ImpactAnalysisResult` — `{ changedNodeId: string, affectedNodes: string[], impactScore: number, explanation: string }`
- [ ] `GraphInsights` — `{ cycles: object[], hotspots: object[], avgCoupling: number, density: number }`
- [ ] `ExportPayload` — `{ format: 'json'|'png', graphData: object, insights: object|null }`

Versioning rules:

- [ ] All v2 contracts follow semver minor bump from v1 contract types
- [ ] Tauri commands include an explicit version field in payload (proposed convention)
- [ ] Contract types in `src/lib/types.ts` mirror exact field names from backend schema
- [ ] Changes to existing contract fields require new minor version, never breaking field names
- [ ] CHANGELOG_CONTRATOS.md updated for every contract change

---

## 4. NFR Targets with Measurable Thresholds

| Metric                         | Threshold                           | Measurement method                       | Status |
| ------------------------------ | ----------------------------------- | ---------------------------------------- | ------ |
| Architecture detection latency | < 3s for 5000 files                 | Benchmark harness in `tests/benchmarks/` | Open   |
| Impact analysis latency        | < 5s for single-node change         | Timer in `src-tauri/src/commands.rs`     | Open   |
| GraphInsights generation       | < 2s for 2000 nodes                 | Benchmark runner                         | Open   |
| Export JSON generation         | < 5s for 5000 nodes                 | Benchmark                                | Open   |
| Export PNG generation          | < 10s for 5000 nodes                | Benchmark                                | Open   |
| UI response to filter change   | < 200ms                             | Chrome DevTools performance              | Open   |
| Memory usage v2 (5000 files)   | < 800 MB                            | Heap profiling via `dhat` or `heaptrack` | Open   |
| SQLite query time (cycles)     | < 500ms                             | `EXPLAIN QUERY PLAN`                     | Open   |
| WAL mode read concurrency      | 0 deadlocks under 10 parallel reads | Integration test                         | Open   |

**Validation**: All NFR thresholds are proposed and must be approved in SDD proposal; then measured in CI against the fixture repo (`fixtures/`) before each RC.

---

## 5. Migration Runbook Checklist

From `docs/ARQUITECTURA_DATOS_V2_V3.md` section 6, migration `003_architecture_and_insights.sql`:

- [ ] Migration script exists at `engine/migrations/003_architecture_and_insights.sql`
- [ ] Script includes: `ALTER TABLE imports ADD COLUMN edge_type TEXT DEFAULT 'import'`
- [ ] Script includes: `CREATE TABLE architecture_detections (...)`
- [ ] Script includes: `CREATE TABLE graph_insights (...)`
- [ ] Script includes: `PRAGMA user_version = 3`
- [ ] Engine auto-detects schema version on startup and runs pending migrations
- [ ] Auto-backup triggered before migration: copy `codeatlas.db` → `codeatlas.db.backup.<timestamp>`
- [ ] Rollback verified: restore from backup and re-run migration works without data loss
- [ ] WAL mode active (`PRAGMA journal_mode=WAL`) before migration runs
- [ ] Migration tested on a DB with existing v1 data (realistic fixture)
- [ ] Migration does NOT break existing `files`, `symbols`, `imports`, `projects` rows (additive-only)
- [ ] `engine/src/db/migrations.rs` updated to register and apply `003_...sql` (to be created)

---

## 6. Degraded-Mode Compatibility Matrix Checklist

| Failure scenario                           | Expected behavior                                                                         | Implementation owner |
| ------------------------------------------ | ----------------------------------------------------------------------------------------- | -------------------- |
| Architecture detector fails (parser error) | Return `{ pattern: 'unknown', confidence: 0.0, evidence: null }`; log error; do not crash | Backend              |
| No IA provider configured                  | Hide AI panel; show banner "Configure API key in Settings"; graph and insights still work | Frontend             |
| IA timeout (> 10s)                         | Return error message in chat panel; allow retry; do not block UI                          | Frontend + Backend   |
| Export PNG fails (large graph)             | Fallback to Export JSON; show warning "PNG too large, exporting JSON instead"             | Frontend             |
| SQLite write fails (disk full)             | Show error dialog; disable scan/save; allow export of existing data                       | Backend              |
| Cycle detection timeout (> 5s)             | Return `{ cycles: [], error: 'timeout' }`; do not block graph render                      | Backend              |
| GraphInsights fails                        | Return `{ cycles: [], hotspots: [], avgCoupling: null, density: null }`; log; continue    | Backend              |
| Tauri command version mismatch             | Show "Update required" banner; do not proceed with stale contract calls                   | Frontend             |

**Degraded-mode test**: Every row above must have a test that simulates the failure and verifies the expected behavior.

---

## 7. Go / No-Go Criteria

### Required for v2 Alpha

- [ ] v1 archived in `openspec/changes/archive/2026-06-01-v1-mvp-core/`
- [ ] v2 SDD phases explore → tasks completed with approval at each gate
- [ ] All 3 core contracts (ArchitectureDetectionResult, ImpactAnalysisResult, GraphInsights) plus ExportPayload implemented and integration-tested (not just unit)
- [ ] NFR thresholds (proposed) validated on fixture with 1000+ files
- [ ] Migration runbook validated end-to-end (dev + prod-like)
- [ ] Degraded-mode matrix tested: all 8 failure scenarios covered
- [ ] CI passes: `cargo test`, `npm run test`, `tauri build --debug`
- [ ] Review budget respected: no PR > 400 changed lines without chained PR approval

### Hard blockers (No-Go)

- Any PR that introduces breaking changes to v1 contract types without version bump
- Any PR that removes or renames existing DB columns from v1 schema
- Any PR that merges v3 scope items (multi-project, snapshots, health timeline)
- Any PR that ships without degraded-mode fallback for the 8 failure scenarios above

---

_Document ready for v2 SDD initiation. Update this checklist as decisions are made during v2 explore/proposal phases._
