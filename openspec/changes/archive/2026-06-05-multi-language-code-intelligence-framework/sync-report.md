# Sync Report — `multi-language-code-intelligence-framework`

## Status

**synced** — File-backed sync of verified-PASS change applied to canonical specs. No blockers. Sync-only (no archive, no commit).

## Summary

| Item                            | Value                                                                                                                                                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Change name                     | `multi-language-code-intelligence-framework`                                                                                                                                                                        |
| Verification                    | PASS (see `verify-report.md`)                                                                                                                                                                                       |
| Sync mode                       | File-backed, additive (two new canonical capabilities)                                                                                                                                                              |
| Canonical specs created         | `openspec/specs/code-intelligence-ir/spec.md`, `openspec/specs/multi-language-dispatch/spec.md`                                                                                                                     |
| Pre-existing canonicals touched | None — `openspec/specs/project-understanding/spec.md` was NOT modified (the proposal explicitly states the change is a capa aditiva; no requirement of the existing `project-understanding` capability was changed) |
| Change folder archived          | No (sync only)                                                                                                                                                                                                      |
| Commit created                  | No                                                                                                                                                                                                                  |
| Source implementation touched   | No                                                                                                                                                                                                                  |
| Active same-domain collisions   | None — see "Active Same-Domain Collision" below                                                                                                                                                                     |
| Destructive sync approvals      | None required (zero `REMOVED` requirements, zero `MODIFIED` requirements, no large rewrites)                                                                                                                        |
| Next recommended phase          | `sdd-archive` for `multi-language-code-intelligence-framework` (only blocker is parent readiness and a `git commit` decision; the change itself is sync-clean)                                                      |

## Domains Synced

| Domain                    | Canonical file                                   | Operation                  |
| ------------------------- | ------------------------------------------------ | -------------------------- |
| `code-intelligence-ir`    | `openspec/specs/code-intelligence-ir/spec.md`    | New canonical spec created |
| `multi-language-dispatch` | `openspec/specs/multi-language-dispatch/spec.md` | New canonical spec created |

Both domain deltas live at `openspec/changes/multi-language-code-intelligence-framework/specs/<domain>/spec.md` as native OpenSpec per-domain delta files (no legacy flat `spec.md`). They contain full spec content (no `## ADDED Requirements` / `## MODIFIED Requirements` / `## RENAMED Requirements` / `## REMOVED Requirements` markers) because the canonical specs for these two capabilities did not exist before this sync. The native helper rule "If canonical spec does not exist, copy the change spec as the new canonical spec" applied for both.

The two change-delta titles (`# Delta Spec: code-intelligence-ir` and `# Delta Spec: multi-language-dispatch`) were normalized to canonical-title style consistent with the existing `openspec/specs/project-understanding/spec.md` (`# Project Understanding MVP Specification`):

- `# Code-Intelligence IR Specification`
- `# Multi-Language Dispatch Specification`

All requirement and scenario bodies were carried over verbatim from the deltas; no semantic edits were applied during sync.

## Canonical Files Updated

| File                                             | Status    | Lines           | Requirements   | Scenarios      |
| ------------------------------------------------ | --------- | --------------- | -------------- | -------------- |
| `openspec/specs/code-intelligence-ir/spec.md`    | Created   | 94              | 6              | 8              |
| `openspec/specs/multi-language-dispatch/spec.md` | Created   | 98              | 5              | 9              |
| `openspec/specs/project-understanding/spec.md`   | Untouched | 542 (unchanged) | 33 (unchanged) | 54 (unchanged) |

The two new files are independent capabilities (no shared section with `project-understanding`). The pre-existing `project-understanding/spec.md` was deliberately not modified; the proposal classifies this change as "capa aditiva" with `## Modified Capabilities: None` and the verify report maps all 6 IR requirements and 5 dispatch requirements exclusively to the new domains.

## ADDED Requirements (11)

All 11 ADDED requirements (6 IR + 5 dispatch) come from the change deltas and were written verbatim into the two new canonical specs:

### `code-intelligence-ir` (6 requirements / 8 scenarios)

1. **IR Shape — LexicalValueKind y Reference** — 2 scenarios
2. **Invariante de Identidad Estable** — 1 scenario
3. **Contrato de Emisión de Reference** — 2 scenarios
4. **Trait Extension sin Duplicación** — 1 scenario
5. **Add-a-Language Contract** — 1 scenario
6. **Single AST Pass** — 1 scenario

### `multi-language-dispatch` (5 requirements / 9 scenarios)

1. **ParserRegistry es el Único Punto de Dispatch** — 2 scenarios
2. **Shim Deprecated para `CodeParser::parse_file`** — 2 scenarios
3. **scan_project usa Registry Una Sola Vez** — 2 scenarios
4. **get_node_outline usa Registry Una Sola Vez** — 1 scenario
5. **Add-a-Language no Toca Dispatch** — 2 scenarios

## MODIFIED Requirements

**None.** No pre-existing canonical requirement was modified. The change proposal declares "Modified Capabilities: None" and explicitly states the new IR/dispatch layer is additive on top of the existing `project-understanding` base. Verified by `wc -l openspec/specs/project-understanding/spec.md` returning 542 (unchanged from the post-`robust-logging-observability`-sync baseline of 542).

## REMOVED Requirements

**None.** No canonical requirement was removed.

## RENAMED Requirements

**None.** The deltas do not declare `## RENAMED Requirements`, and the helper blocks on this case anyway. No blocker triggered.

## Destructive Sync Approvals

**Not required.** Zero `REMOVED` requirements, zero `MODIFIED` requirements, no `## RENAMED Requirements` block. The sync is pure additive creation of two new canonical capability files. No destructive approval workflow was triggered.

## Active Same-Domain Collision

**No collision for this change.** The other active change `outline-parser-abstraction` targets the `project-understanding` domain (delta at `openspec/changes/outline-parser-abstraction/specs/project-understanding/spec.md`). The selected change targets `code-intelligence-ir` and `multi-language-dispatch` only — disjoint domain set from `outline-parser-abstraction`, so there is no same-domain conflict for sync ordering.

**For the parent / archive agent:** when `outline-parser-abstraction` is later synced, that operation is independent of this sync. If it also is a pure additive append to `project-understanding`, no coordination with this change is needed. If it turns out to be destructive, then a follow-up review will be required at that time and the present sync of `multi-language-code-intelligence-framework` will not be impacted.

## Guardrail Notes

| Guardrail                                          | Result                                                                                                                                              |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Legacy flat spec detection                         | Not triggered — the change uses native `specs/<domain>/spec.md` per-domain deltas                                                                   |
| `## RENAMED Requirements` block                    | Not present — no blocker                                                                                                                            |
| Active same-domain collision warning               | None for this change; the other active change `outline-parser-abstraction` targets a different domain                                               |
| Destructive `REMOVED` / large `MODIFIED` deltas    | None present; no approval required                                                                                                                  |
| File modification scope (no source implementation) | Respected — only `openspec/specs/code-intelligence-ir/spec.md`, `openspec/specs/multi-language-dispatch/spec.md`, and this sync report were written |
| Archive / commit performed by sync                 | No — sync only                                                                                                                                      |
| `rules.sync` from `openspec/config.yaml`           | No `rules.sync` block declared in `openspec/config.yaml`; nothing to apply                                                                          |
| Title normalization (delta → canonical)            | Applied — `# Delta Spec: <id>` → `# <Capability Name> Specification` to match the existing `project-understanding` style                            |

## Validation Commands / Checks Performed

This sync is a markdown-only text operation; no Rust/TS toolchain was invoked. Manual validation:

| Check                                                                                                                          | Result                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------- |
| Both canonical files exist and are non-empty                                                                                   | OK — `code-intelligence-ir/spec.md` = 94 lines, `multi-language-dispatch/spec.md` = 98 lines |
| Requirement count matches delta content                                                                                        | OK — `code-intelligence-ir` = 6 requirements, `multi-language-dispatch` = 5 requirements     |
| Scenario count matches delta content                                                                                           | OK — `code-intelligence-ir` = 8 scenarios, `multi-language-dispatch` = 9 scenarios           |
| `## ADDED Requirements` / `## MODIFIED Requirements` / `## RENAMED Requirements` / `## REMOVED Requirements` markers in deltas | None — sync was a new-canonical creation, not an in-place modification                       |
| Pre-existing canonical `project-understanding/spec.md` unchanged                                                               | OK — `wc -l` = 542 (matches post-`robust-logging-observability`-sync baseline)               |
| Source implementation files unchanged                                                                                          | OK — no files under `src/`, `src-tauri/src/`, or `engine/src/` were touched by this sync     |
| No `git` commands executed (no commit, no archive)                                                                             | OK — `.git/` was not modified                                                                |
| Sync report file written                                                                                                       | OK — `openspec/changes/multi-language-code-intelligence-framework/sync-report.md` written    |

## Next Recommended Phase

`sdd-archive` for `multi-language-code-intelligence-framework` once the parent is ready to:

1. Verify that the user (interactive mode) explicitly approves the next phase. Per session preflight: in interactive mode, "complete only the current SDD phase" and "do not start the next SDD phase unless the current user turn explicitly approves that next phase". Words like "continue" / "dale" / "go on" approve only the immediate next phase.
2. Commit the two new canonical specs together with the previously committed implementation (PR-A, PR-B, PR-C via `ff372bd` + the verify-fix diff on the working tree).
3. Move `openspec/changes/multi-language-code-intelligence-framework/` to `openspec/changes/archive/2026-06-05-multi-language-code-intelligence-framework/` (or the date the parent decides) per the archive convention observed in the existing archive layout (e.g. `2026-06-04-robust-logging-observability`).
4. Update `openspec/README.md` to list this change in "Cambios archivados".

The archive agent should re-read both new canonical specs before moving the change, and should refuse to archive if either file has been edited in a way that diverges from the delta content captured in this sync report.

## Residual Risks (carryover, not introduced by sync)

| #   | Risk / Carryover                                                                                                                                                                                                                                                                                                                       | Source                | Sync impact                                                                                                                                       |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Pre-existing `SymbolInfo.id` uses `uuid::Uuid::new_v4()` in `typescript.rs:287` and `rust.rs:74`; the new `code-intelligence-ir` spec requires stable composite `(file_id, kind, name, range)` IDs. WARN #2 in `verify-report.md`.                                                                                                     | Pre-existing v1/v2/v3 | None — sync carries the spec text faithfully; the gap is in the implementation, not the canonical spec. A follow-up change should reconcile this. |
| 2   | Pre-existing `cargo clippy --all-targets -- -D warnings` failures in `engine/` (bench/test targets). WARN #1 in `verify-report.md`.                                                                                                                                                                                                    | Pre-existing          | None — sync is markdown-only; orthogonal to clippy gate.                                                                                          |
| 3   | `parse_file` shim is preserved (deprecation only). The dispatch spec declares its removal as "follow-up change".                                                                                                                                                                                                                       | Proposal              | None — spec is honest about scope; sync reflects this.                                                                                            |
| 4   | Frontend vitest Tauri-invoke failures (WARN #5) and missing root `README.md` (WARN #4) are pre-existing.                                                                                                                                                                                                                               | Pre-existing          | None — sync is markdown-only; orthogonal.                                                                                                         |
| 5   | Verify-fix diff is uncommitted on working tree (per `verify-report.md`). The implementation that the new canonical specs describe is in the merged PRs (`ff372bd`) and the uncommitted verify-fix diff that closes the prior CRITICAL. Sync does not commit; the archive phase will need to decide how to handle the uncommitted diff. | Verify report         | None for sync; flag for the parent / archive agent.                                                                                               |
