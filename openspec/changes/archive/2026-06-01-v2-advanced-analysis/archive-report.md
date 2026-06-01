# Archive Report — v2-advanced-analysis

**Date:** 2026-06-01  
**Archive Status:** ✅ PASS  
**Archived Path:** `openspec/changes/archive/2026-06-01-v2-advanced-analysis/`

---

## Artifacts Read

| Artifact       | Path                                                                        |
| -------------- | --------------------------------------------------------------------------- |
| Proposal       | `openspec/changes/v2-advanced-analysis/proposal.md`                         |
| Spec (delta)   | `openspec/changes/v2-advanced-analysis/specs/project-understanding/spec.md` |
| Design         | `openspec/changes/v2-advanced-analysis/design.md`                           |
| Tasks          | `openspec/changes/v2-advanced-analysis/tasks.md`                            |
| Verify Report  | `openspec/changes/v2-advanced-analysis/verify-report.md`                    |
| Apply Progress | `openspec/changes/v2-advanced-analysis/apply-progress.md`                   |

---

## Canonical Spec Sync

| Domain                   | Path                                          | Action                     |
| ------------------------ | --------------------------------------------- | -------------------------- |
| `project-understanding`  | `openspec/specs/project-understanding/spec.md` | Updated (ADDED requirements) |

### ADDED Requirements (9)

| # | Requirement Name |
|---|---|
| 1 | Architecture Detection with Evidence |
| 2 | Impact Analysis |
| 3 | Graph Insights (Cycles and Hotspots) |
| 4 | Exportable Analysis Evidence |
| 5 | v2 Analytical Views and Persistent Filters |
| 6 | v2 Contract Compatibility |
| 7 | Additive v2 Data Migration |
| 8 | i18n Foundation for Spanish Catalog |
| 9 | v3 Scope Exclusion Enforcement |

### MODIFIED Requirements

None.

### REMOVED Requirements

None.

### Destructive Merge

No destructive merge performed. All changes were ADDED requirements appended to the canonical spec under a `## v2 Additions — v2-advanced-analysis (archived 2026-06-01)` section header.

### Same-Domain Active Change Warnings

None. The only other change touching `project-understanding` (v1-mvp-core) was already archived on 2026-06-01.

---

## Exceptions Deferred (post-v2 hardening)

These two gaps were documented in the verify report as non-blocking for Alpha:

| # | Gap | Planned Resolution |
|---|---|---|
| 1 | NFR benchmarks — scaffold only, no fixtures or real measurements | Create 1000+ file fixture and run benchmarks against thresholds before Beta |
| 2 | Degraded-mode matrix — 4/8 scenarios tested (backend); 4 frontend/IA pending | Add integration tests for PNG fallback via mock, contract mismatch, IA not configured, IA timeout |

---

## Move to Archive

```
openspec/changes/v2-advanced-analysis/
  → openspec/changes/archive/2026-06-01-v2-advanced-analysis/
```

All 9 artifacts preserved for audit trail.

---

## Memory

No Engram persistence available in this session. Archive traceability maintained via filesystem artifacts.

---

_Archivo completado con sync canónico exitoso. Gaps de hardening documentados para la siguiente etapa._
