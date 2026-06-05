# Archive Report — `multi-language-code-intelligence-framework`

## Status

✅ **ARCHIVED**

The SDD change `multi-language-code-intelligence-framework` is verified (PASS), file-backed–synced, and moved to the dated archive. No source implementation files were touched during archive. No commit was performed.

## Summary

| Item                            | Value                                                                                                          |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Change name                     | `multi-language-code-intelligence-framework`                                                                   |
| Verification                    | PASS (`verify-report.md`) — prior CRITICAL #1 (double-parse in `scan_project`) is **closed**                   |
| Sync                            | PASS (`sync-report.md`) — additive-only, two new canonical capabilities                                        |
| Source implementation committed | Yes — PR-A/B/C stacked-to-main via `ff372bd`; verify-fix diff **uncommitted on working tree** (carryover WARN) |
| Destructive merge performed     | No — sync was purely additive (0 MODIFIED, 0 REMOVED, 0 RENAMED)                                              |
| Same-domain collision           | None — `outline-parser-abstraction` (the other active change) targets `project-understanding`, disjoint        |
| Git commit created by archive   | No                                                                                                             |
| Source files touched by archive | No — only OpenSpec artifacts                                                                                   |

## Archive Destination

Change artifacts moved from:

- `openspec/changes/multi-language-code-intelligence-framework/`

To:

- `openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/`

Date used: `2026-06-05` (today, per the archive convention `YYYY-MM-DD-<change>` established by the four existing entries: `2026-06-01-v1-mvp-core`, `2026-06-01-v2-advanced-analysis`, `2026-06-01-v3-collaboration-platform`, `2026-06-04-robust-logging-observability`).

## Artifacts Read (preconditions)

| Artifact                                                                                  | Status   | Notes                                                                                              |
| ----------------------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `openspec/changes/multi-language-code-intelligence-framework/proposal.md`                 | ✅ Read  | 95 lines; declares new capabilities `code-intelligence-ir` + `multi-language-dispatch`; MODIFIED: None |
| `openspec/changes/multi-language-code-intelligence-framework/specs/code-intelligence-ir/spec.md`     | ✅ Read | Full spec content, no delta markers (new canonical created by sync)                                |
| `openspec/changes/multi-language-code-intelligence-framework/specs/multi-language-dispatch/spec.md` | ✅ Read | Full spec content, no delta markers (new canonical created by sync)                                |
| `openspec/changes/multi-language-code-intelligence-framework/design.md`                   | ✅ Read  | 254 lines; 8 architecture decisions, 3-PR chained plan, per-PR rollback, single AST pass invariant |
| `openspec/changes/multi-language-code-intelligence-framework/tasks.md`                    | ✅ Read  | 173 lines; 13 tasks A.1–A.4 / B.1–B.5 / C.1–C.4; **0 unchecked `- [ ]` markers** (uses ✅ DONE)   |
| `openspec/changes/multi-language-code-intelligence-framework/verify-report.md`            | ✅ Read  | 337 lines; **PASS**; no unresolved `FAIL` / `BLOCKED` / `CRITICAL`; previous CRITICAL #1 closed    |
| `openspec/changes/multi-language-code-intelligence-framework/sync-report.md`              | ✅ Read  | 139 lines; **synced**; purely additive; 11 ADDED requirements / 17 scenarios                      |
| `openspec/changes/multi-language-code-intelligence-framework/apply-progress-pr-b.md`      | ✅ Read  | PR-B TDD cycle evidence                                                                             |
| `openspec/changes/multi-language-code-intelligence-framework/apply-progress-pr-c.md`      | ✅ Read  | PR-C TDD cycle evidence (retroactive)                                                              |
| `openspec/changes/multi-language-code-intelligence-framework/verify-report-pr-a.md`       | ✅ Read  | PR-A evidence trail                                                                                 |
| `openspec/changes/multi-language-code-intelligence-framework/explore.md`                  | ✅ Read  | Pre-proposal exploration (not required for archive, but present in the folder)                      |
| `artifacts/sdd-sync-multi-language-code-intelligence-framework.md`                         | ✅ Read  | Sync-phase result file (parent-provided input)                                                      |
| `openspec/config.yaml`                                                                    | ✅ Read  | `scope.active_change: null` (v3 not initiated); `testing.gates` enumerated; `phase_rules.apply.enforce_strict_tdd: true` |
| `openspec/README.md`                                                                      | ✅ Read  | Archive convention confirmed; "Cambios archivados" list present                                     |

## Sync Result (carryover from sync phase)

### Canonical Files Created

| File                                             | Operation   | Lines | Requirements | Scenarios |
| ------------------------------------------------ | ----------- | ----- | ------------ | --------- |
| `openspec/specs/code-intelligence-ir/spec.md`    | Created     | 94    | 6            | 8         |
| `openspec/specs/multi-language-dispatch/spec.md` | Created     | 98    | 5            | 9         |
| `openspec/specs/project-understanding/spec.md`   | Untouched   | 542   | 33 (unchanged) | 54 (unchanged) |

### ADDED Requirements (11)

**`code-intelligence-ir` (6 requirements):**

1. `IR Shape — LexicalValueKind y Reference`
2. `Invariante de Identidad Estable`
3. `Contrato de Emisión de Reference`
4. `Trait Extension sin Duplicación`
5. `Add-a-Language Contract`
6. `Single AST Pass`

**`multi-language-dispatch` (5 requirements):**

1. `ParserRegistry es el Único Punto de Dispatch`
2. `Shim Deprecated para CodeParser::parse_file`
3. `scan_project usa Registry Una Sola Vez`
4. `get_node_outline usa Registry Una Sola Vez`
5. `Add-a-Language no Toca Dispatch`

### MODIFIED Requirements

**None.**

### REMOVED Requirements

**None.**

### RENAMED Requirements

**None.** (No `## RENAMED Requirements` block declared in deltas; helper blocks on this case anyway.)

## Destructive Merge Approvals / Blockers

**Not required** — sync was purely additive. No `REMOVED` requirements, no `MODIFIED` requirements, no large rewrites. The Destructive Merge Guard was not triggered. Per the SDD status contract rule "Verification alone is not approval for destructive canonical spec changes", this archive does **not** carry a destructive approval, and none is needed.

## Final Task Completion Gate

Re-read `openspec/changes/multi-language-code-intelligence-framework/tasks.md` immediately before archive. Search for `^\s*- \[ \]` (unchecked implementation task boxes):

- `grep -cE '^\s*-\s+\[\s\]' tasks.md` → **0** matches
- `grep -cE '^\s*-\s+\[x\]' tasks.md` → **0** matches (tasks.md uses `**Status**: ✅ DONE` markers, not Markdown checkboxes)

All 13 implementation tasks (A.1–A.4, B.1–B.5, C.1–C.4) are functionally complete per `verify-report.md` §"Task Completion Status" with explicit evidence per task (commit hashes, test files, line counts).

**Gate result**: PASS. No stale-checkbox reconciliation was needed. No `- [ ]` implementation task boxes remain; no partial-archive approval is required.

## Active Same-Domain Collision Warning

**None for this change.** The other active change `outline-parser-abstraction` targets the `project-understanding` domain only. The archived change targets `code-intelligence-ir` + `multi-language-dispatch` only — disjoint domain set. No coordination needed for `outline-parser-abstraction` archive (when its turn comes, it operates on `project-understanding` only, which this change explicitly did not modify).

## Archived Artifacts

After the move, the archive folder contains the full set of in-flight artifacts (anything the change had at archive time):

- `proposal.md` (95 lines)
- `specs/code-intelligence-ir/spec.md` (94 lines)
- `specs/multi-language-dispatch/spec.md` (98 lines)
- `design.md` (254 lines)
- `tasks.md` (173 lines)
- `apply-progress-pr-b.md` (78 lines) — per-PR TDD evidence
- `apply-progress-pr-c.md` (69 lines) — per-PR TDD evidence
- `verify-report.md` (337 lines) — PASS
- `verify-report-pr-a.md` (130 lines) — PR-A evidence
- `sync-report.md` (139 lines) — synced
- `archive-report.md` (this file)
- `explore.md` (270 lines) — pre-proposal exploration (kept for audit trail)

**Cosmetic gap (not a blocker, carryover from WARN #3 in verify-report):** the standard `apply-progress.md` filename is not present — this change has per-PR `apply-progress-pr-b.md` and `apply-progress-pr-c.md` artifacts. The two together cover the full PR-A/B/C lifecycle. Recommend a follow-up to either rename the union of the two per-PR artifacts to `apply-progress.md` or add a short pointer artifact. **Non-blocking** for archive.

## Residual Risks (carryover from sync + verify reports, not introduced by archive)

| #   | Risk                                                                                                            | Source                         | Archive impact |
| --- | --------------------------------------------------------------------------------------------------------------- | ------------------------------ | -------------- |
| 1   | `SymbolInfo::id` uses `uuid::Uuid::new_v4()` in `typescript.rs:287` and `rust.rs:74`; the new `code-intelligence-ir` spec requires stable composite `(file_id, kind, name, range)` IDs. | Verify WARN #2 (pre-existing v1/v2/v3) | None — gap is in implementation, not in archived specs. Open follow-up change. |
| 2   | `cargo clippy --all-targets -- -D warnings` fails on pre-existing bench/test targets. Repo-standard gate `cargo clippy --lib -- -D warnings` is clean.                       | Verify WARN #1 (pre-existing)  | None — gate set is `--lib`; pre-existing bench/test cruft. |
| 3   | `parse_file` shim is preserved (deprecation only). Removal is a follow-up change.                                                                                    | Proposal / design              | None — spec is honest about scope. |
| 4   | Verify-fix diff is **uncommitted on working tree** (per verify-report). The implementation that the new canonical specs describe is in the merged PRs (`ff372bd`) plus the uncommitted verify-fix diff. | Verify report                  | Archive did not commit. The user/parent must decide when to land the verify-fix commit. Archive timing is independent of that decision. |
| 5   | Frontend vitest Tauri-invoke failures (WARN #5) and missing root `README.md` (WARN #4) are pre-existing.                                                                | Verify report (pre-existing)   | None — pre-existing on `main`. |
| 6   | Unmerged local branches `feat/parser-ir-pr-c-core`, `feat/parser-ir-pr-c-docs`, `feat/parser-ir-pr-b-rust`, `feat/parser-ir-pr-b-ts` carryover from PR-B/PR-C work.            | Verify report                  | Cosmetic — recommend cleanup post-archive. |
| 7   | Cosmetic gap: no standard `apply-progress.md` filename (per-PR files only).                                                                                          | Verify WARN #3                 | Cosmetic only; per-PR artifacts cover the lifecycle. |

## Structured Status and `actionContext` Findings

```yaml
status: archived
executive_summary: >
  File-backed archive executed for multi-language-code-intelligence-framework.
  Sync was purely additive (two new canonical capabilities: code-intelligence-ir
  with 6 requirements / 8 scenarios, multi-language-dispatch with 5 requirements
  / 9 scenarios). 0 MODIFIED, 0 REMOVED, 0 RENAMED. No destructive merge approval
  required. Verify report is PASS with the prior CRITICAL #1 (double-parse in
  scan_project) closed by the verify-fix diff. No - [ ] implementation task
  boxes remain. Same-domain collision check is clean (outline-parser-abstraction
  targets project-understanding only). Source implementation untouched by
  archive. No commit performed. Folder moved to
  openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/.
artifacts_archived:
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
canonical_specs_created:
  - openspec/specs/code-intelligence-ir/spec.md
  - openspec/specs/multi-language-dispatch/spec.md
canonical_specs_modified: []
source_implementation_touched: false
commit_performed: false
destructive_merge_approval_required: false
unchecked_implementation_task_boxes: 0
stale_checkbox_reconciliation: not_needed
partial_archive_approval: not_needed
actionContext:
  mode: repo-local
  workspaceRoot: /home/xeworg/Proyectos/codeatlas
  allowedEditRoots:
    - /home/xeworg/Proyectos/codeatlas
  archive_path_within_workspace: true
  warnings: []
relationships:
  dependsOn: []
  supersedes: []
  amends: []
  conflictsWith: []
  sameDomainActiveChanges: []
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

## `skill_resolution`

`paths-injected` — the parent provided the SDD status contract in-prompt and the explicit change selection (`multi-language-code-intelligence-framework`) and the explicit sync-result file path. The executor did not need to fall back to the project override, global install, or registry paths for the status contract or skill resolution. No `## Skills to load before work` paths were injected by the parent for this phase, so no `SKILL.md` files were loaded. The parent should pass indexed paths next time if a non-default executor skill is desired for archive.

## Final State

`multi-language-code-intelligence-framework` is **closed**. No further SDD work is required for this change unless the user requests follow-ups (e.g. UUID-to-composite-key migration for `SymbolInfo::id`, real Python/Go/Java parsers, shim removal, or root `README.md` creation linking the C.4 author guide).

The active SDD queue now contains:

- `outline-parser-abstraction` (the other active change, targets `project-understanding` only — disjoint from this change's domains; not touched by this archive)
