# SDD Archive Result — `multi-language-code-intelligence-framework`

**Phase:** `sdd-archive`
**Change:** `multi-language-code-intelligence-framework`
**Status:** ✅ **archived**
**Archived at:** 2026-06-05
**Run by:** archive executor (subagent)

## TL;DR

File-backed archive of the verified-PASS, sync-clean change `multi-language-code-intelligence-framework` executed cleanly. The change folder was moved from `openspec/changes/multi-language-code-intelligence-framework/` to `openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/` (per the established `YYYY-MM-DD-<change>` convention). The `archive-report.md` was written **inside** the change folder before the move, so it travels with the change into the archive. The two new canonical specs (`openspec/specs/code-intelligence-ir/spec.md` + `openspec/specs/multi-language-dispatch/spec.md`) created during the sync phase remain in place. The pre-existing canonical `openspec/specs/project-understanding/spec.md` was **not** modified at any point. No source implementation file was touched by archive. No commit was performed.

The other active change `outline-parser-abstraction` is untouched and remains active; its domain (`project-understanding`) is disjoint from this change's domains, so the archive does not affect its sync order.

## Destination Path

```
openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/
```

Date used: `2026-06-05` (today, per the convention established by the four prior archives: `2026-06-01-v1-mvp-core`, `2026-06-01-v2-advanced-analysis`, `2026-06-01-v3-collaboration-platform`, `2026-06-04-robust-logging-observability`).

## Archived Artifacts

The archive folder contains the full set of in-flight artifacts (12 files + the `specs/` subdirectory):

| Artifact                                | Lines | Notes                                                                                                                   |
| --------------------------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------- |
| `proposal.md`                           | 95    | Intent, scope, capabilities (new: `code-intelligence-ir` + `multi-language-dispatch`; MODIFIED: None), success criteria |
| `specs/code-intelligence-ir/spec.md`    | 94    | Full domain spec (new canonical, 6 requirements / 8 scenarios)                                                          |
| `specs/multi-language-dispatch/spec.md` | 98    | Full domain spec (new canonical, 5 requirements / 9 scenarios)                                                          |
| `design.md`                             | 254   | 8 architecture decisions, 3-PR chained plan (A→B→C), per-PR rollback                                                    |
| `tasks.md`                              | 173   | 13 tasks A.1–A.4 / B.1–B.5 / C.1–C.4; 0 unchecked `- [ ]` boxes                                                         |
| `apply-progress-pr-b.md`                | 78    | Per-PR TDD cycle evidence (PR-B)                                                                                        |
| `apply-progress-pr-c.md`                | 69    | Per-PR TDD cycle evidence (PR-C, retroactive)                                                                           |
| `verify-report.md`                      | 337   | **PASS** — prior CRITICAL #1 (double-parse in `scan_project`) **closed**                                                |
| `verify-report-pr-a.md`                 | 130   | PR-A evidence trail                                                                                                     |
| `sync-report.md`                        | 139   | **synced** — purely additive, 11 ADDED / 0 MODIFIED / 0 REMOVED                                                         |
| `archive-report.md`                     | 216   | This archive's own report (written before the folder move)                                                              |
| `explore.md`                            | 270   | Pre-proposal exploration (kept for audit trail)                                                                         |

**Cosmetic gap (carryover from verify WARN #3, non-blocking):** no standard `apply-progress.md` filename; the change has per-PR `apply-progress-pr-b.md` + `apply-progress-pr-c.md` artifacts. Recommend a follow-up to either rename the union of the two per-PR artifacts to `apply-progress.md` or add a short pointer artifact.

## Canonical Specs (carried over from sync)

| File                                             | Status    | Lines | Requirements   | Scenarios      |
| ------------------------------------------------ | --------- | ----- | -------------- | -------------- |
| `openspec/specs/code-intelligence-ir/spec.md`    | Created   | 94    | 6              | 8              |
| `openspec/specs/multi-language-dispatch/spec.md` | Created   | 98    | 5              | 9              |
| `openspec/specs/project-understanding/spec.md`   | Untouched | 542   | 33 (unchanged) | 54 (unchanged) |

### ADDED Requirements (11 total)

**`code-intelligence-ir` (6):**

1. `IR Shape — LexicalValueKind y Reference`
2. `Invariante de Identidad Estable`
3. `Contrato de Emisión de Reference`
4. `Trait Extension sin Duplicación`
5. `Add-a-Language Contract`
6. `Single AST Pass`

**`multi-language-dispatch` (5):**

1. `ParserRegistry es el Único Punto de Dispatch`
2. `Shim Deprecated para CodeParser::parse_file`
3. `scan_project usa Registry Una Sola Vez`
4. `get_node_outline usa Registry Una Sola Vez`
5. `Add-a-Language no Toca Dispatch`

### MODIFIED / REMOVED / RENAMED

**None.** Purely additive sync; no destructive merge approval was required. The Destructive Merge Guard was not triggered.

## Preconditions Verified

| Check                                          | Result                                                                                                              |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Verify report present and PASS                 | ✅ `verify-report.md` present; status PASS; no unresolved `FAIL` / `BLOCKED` / `CRITICAL`; prior CRITICAL #1 closed |
| Sync report present and clean                  | ✅ `sync-report.md` present; status **synced**; purely additive; canonical files match delta content                |
| Sync fallback required?                        | ❌ No — sync was completed in the previous phase and the result file was provided                                   |
| Required artifacts present                     | ✅ `proposal.md`, `specs/<domain>/spec.md` (×2), `design.md`, `tasks.md`, `verify-report.md`, `sync-report.md`      |
| Legacy flat `spec.md` (no per-domain specs)    | ❌ Not triggered — change uses native `specs/<domain>/spec.md` per-domain deltas                                    |
| Unchecked `- [ ]` implementation tasks         | ✅ 0 matches in `tasks.md` (uses `**Status**: ✅ DONE` markers instead)                                             |
| Stale-checkbox reconciliation needed           | ❌ No — every implementation task is complete and recorded with evidence in the verify report                       |
| Partial-archive approval needed                | ❌ No — all required artifacts are present; no missing proposal/spec/design                                         |
| Active same-domain collision                   | ✅ None — `outline-parser-abstraction` targets `project-understanding` only; this change is disjoint                |
| Destructive sync approval                      | ❌ Not required — purely additive sync                                                                              |
| `actionContext.allowedEditRoots` covers target | ✅ `/home/xeworg/Proyectos/codeatlas` is in the allowed edit roots                                                  |

## Constraints Honored

- ✅ Archived **only after** confirming sync completed successfully (sync report read, canonical specs verified on disk)
- ✅ Moved the change directory using the existing `openspec/changes/archive/YYYY-MM-DD-<change>/` convention
- ✅ Produced `archive-report.md` **inside** the change folder before moving (so the report travels with the change)
- ✅ Did **not** perform any `git commit`
- ✅ Did **not** modify any source implementation file outside OpenSpec artifacts (only `openspec/changes/`, `openspec/specs/`, and `artifacts/` were touched; `engine/src/`, `src-tauri/src/`, and `src/` are untouched by archive — the working-tree modifications visible in `git status` are pre-existing from the verify-fix diff and prior agent-config changes)
- ✅ Did **not** touch `outline-parser-abstraction` (the other active change)
- ✅ Interactive mode: did **not** auto-advance to a next SDD phase; this archive task is the final SDD phase for this change

## Residual Risks (carryover — not introduced by archive)

These are WARN items from the verify report, not archive-induced. They do **not** block archive and are listed for the parent/orchestrator's awareness.

1. **`SymbolInfo::id` UUID vs stable composite key (WARN #2 from verify)** — `typescript.rs:287` and `rust.rs:74` use `uuid::Uuid::new_v4()`. The new `code-intelligence-ir` spec requires stable composite `(file_id, kind, name, range)` IDs. Pre-existing from v1/v2/v3, not introduced by this change. Spec is faithful; the gap is in implementation. Recommend a follow-up change.
2. **Verify-fix diff is uncommitted on working tree** (per verify report) — the implementation that the new canonical specs describe includes the uncommitted fix that closes the prior CRITICAL. Archive does not commit; the parent/user must decide when to land this diff. Archive timing is independent of that decision.
3. **`parse_file` shim preserved as `#[deprecated]`** (spec declares removal as a follow-up). Faithfully reflected in the canonical spec.
4. **Pre-existing `cargo clippy --all-targets -- -D warnings` failures (WARN #1)** on bench/test targets. Outside the configured gate set (`--lib -- -D warnings` is clean). Not an archive concern.
5. **Pre-existing vitest Tauri-invoke failures (WARN #5)** and **missing root `README.md` (WARN #4)** — unrelated to this change.
6. **Cosmetic gap: no standard `apply-progress.md` (WARN #3)** — per-PR `apply-progress-pr-b.md` + `apply-progress-pr-c.md` together cover the full PR-A/B/C lifecycle. Non-blocking; recommend follow-up consolidation.
7. **Unmerged local branches** `feat/parser-ir-pr-c-core`, `feat/parser-ir-pr-c-docs`, `feat/parser-ir-pr-b-rust`, `feat/parser-ir-pr-b-ts` carryover from PR-B/PR-C work. Recommend cleanup post-archive (cosmetic).

## `skill_resolution`

`paths-injected` — the parent provided the resolved SDD status contract in-prompt (artifact store `openspec`, workspace root `/home/xeworg/Proyectos/codeatlas`, change `multi-language-code-intelligence-framework`) and the explicit sync-result file path. The executor did not need to fall back to the project override, global install, or registry paths for the status contract or skill resolution. No `## Skills to load before work` paths were injected by the parent for this phase, so no `SKILL.md` files were loaded. The parent should pass indexed paths next time if a non-default executor skill is desired for archive.

## Next Recommended Phase

None for this change. The change is **closed**. The active SDD queue now contains only `outline-parser-abstraction` (targeting `project-understanding` only; disjoint from this change's domains; untouched by this archive).

The parent should:

1. (Optional) Update `openspec/README.md` "Cambios archivados" list to include `2026-06-05-multi-language-code-intelligence-framework/`.
2. (Decide) When to commit the working-tree changes (verify-fix diff + new canonical specs + archive folder move + per-PR progress artifacts) — this is independent of archive timing.
3. (Decide) When to run `sdd-archive` for `outline-parser-abstraction` once it is verify-PASS and synced.
4. (Decide) Whether to address the carryover follow-ups (UUID stability, bench clippy, vitest Tauri mocks, root README, per-PR artifact consolidation, unmerged branch cleanup).

---

## Phase Envelope (for orchestrator)

```yaml
status: archived
executive_summary: >
  File-backed archive executed for multi-language-code-intelligence-framework.
  Folder moved from openspec/changes/multi-language-code-intelligence-framework/
  to openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/.
  archive-report.md was written inside the change folder before the move and
  travels with the change. The two new canonical capabilities (code-intelligence-ir
  with 6 requirements / 8 scenarios, multi-language-dispatch with 5 requirements /
  9 scenarios) were created during the sync phase and remain in place; the
  pre-existing project-understanding canonical was not modified. No source
  implementation file was touched by archive. No commit performed. Verify report
  is PASS with the prior CRITICAL #1 closed. No - [ ] implementation task boxes
  remain. Same-domain collision check is clean. The other active change
  (outline-parser-abstraction) was not touched and remains active.
artifacts:
  archived:
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/proposal.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/specs/code-intelligence-ir/spec.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/specs/multi-language-dispatch/spec.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/design.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/tasks.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/apply-progress-pr-b.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/apply-progress-pr-c.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/verify-report.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/verify-report-pr-a.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/sync-report.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/explore.md
    - openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/archive-report.md
  created_by_sync_earlier_and_retained:
    - openspec/specs/code-intelligence-ir/spec.md
    - openspec/specs/multi-language-dispatch/spec.md
  NOT_modified:
    - openspec/specs/project-understanding/spec.md
  NOT_touched_source_implementation: true
  NOT_committed: true
next_recommended: none_for_this_change
risks:
  - SymbolInfo::id UUID vs composite key (pre-existing; spec carries the requirement, implementation does not yet) — carryover
  - Verify-fix diff uncommitted on working tree — carryover, archive did not commit
  - Pre-existing clippy --all-targets bench/test failures — carryover
  - Pre-existing vitest Tauri-invoke failures — carryover
  - parse_file shim removal deferred to follow-up change — carryover
  - Cosmetic gap: no standard apply-progress.md (per-PR files only) — carryover
  - Unmerged local feature branches carryover — cosmetic, not blocking
skill_resolution: paths-injected
```
