# Delta for Project Understanding MVP

## ADDED Requirements

### Requirement: H1 Hardening Gates from v2 Carry-Over

Before closing H1, the system MUST complete and verify all three carry-over gates inherited from archived v2 exceptions.

#### Scenario: Gate 1 — Real fixture and NFR benchmark evidence

- GIVEN v3 H1 is in progress
- WHEN NFR validation is executed
- THEN the system MUST run benchmarks on a real fixture of 1000+ files
- AND the system MUST record measurable evidence for scan, graph insights, and export timing against declared thresholds

#### Scenario: Gate 2 — Remaining degraded-mode frontend/IA scenarios

- GIVEN degraded-mode validation for v2/v3 compatibility
- WHEN test suites run
- THEN the system MUST cover pending frontend/IA scenarios: PNG fallback via mock, contract mismatch handling, AI not configured, and AI timeout
- AND each scenario MUST verify non-crashing fallback behavior and user-visible error/warning states

#### Scenario: Gate 3 — App-level wiring T5.6

- GIVEN analytical components are wire-ready
- WHEN H1 is completed
- THEN `App.tsx` MUST integrate `AnalyticsViewSelector`, `ArchitectureCard`, `ImpactPanel`, and `InsightsPanel`
- AND users MUST be able to reach those views through the main app flow without manual patching

### Requirement: H1 Multi-Project Workspaces Foundation

The system MUST support workspace-level organization for multiple projects in v3 H1.

#### Scenario: Workspace groups projects

- GIVEN a user with more than one scanned project
- WHEN the user manages workspace context
- THEN the system MUST allow associating projects to a workspace boundary
- AND project-level analysis data MUST remain isolated per project identity

### Requirement: H2 Collaboration Baseline

The system MUST provide local-first collaboration primitives in v3 H2 through snapshots and annotations.

#### Scenario: Snapshot creation and retrieval

- GIVEN an analyzed project state
- WHEN the user creates a snapshot with a label
- THEN the system MUST persist a snapshot artifact that can be listed and reloaded later

#### Scenario: Node annotations are persisted

- GIVEN a graph node in a project context
- WHEN the user adds a comment/annotation
- THEN the system MUST persist the annotation with author and timestamp metadata
- AND the annotation MUST be retrievable in later sessions

### Requirement: H3 Executive Insight Surfaces

The system MUST provide executive-level views in v3 H3 with health timeline and architecture comparison outputs.

#### Scenario: Health timeline is available by period

- GIVEN historical health records for a project or workspace
- WHEN the user requests a time window
- THEN the system MUST return a timeline including overall and component architecture health metrics

#### Scenario: Comparative architecture view

- GIVEN at least two snapshots
- WHEN the user requests a comparison
- THEN the system MUST return a diff-capable representation suitable for C4-assisted and snapshot comparison views

### Requirement: V3 Contract and Migration Consistency

The system MUST keep v3 contracts and migrations consistent with documented plans while preserving additive compatibility with v1/v2 data.

#### Scenario: Planned contracts are exposed without v1/v2 breakage

- GIVEN a v3-capable client
- WHEN it invokes v3 collaboration/executive commands
- THEN responses MUST match documented v3 contract families (`Snapshot`, `Comment`, `SharedView`, `HealthScoreTimeline`, `ExecutiveArchitectureSummary`)
- AND existing v1/v2 command flows MUST remain functional without required breaking renames/removals

#### Scenario: Planned migrations are additive and recoverable

- GIVEN a database containing v1/v2 data
- WHEN migrations `004_workspace_and_snapshots.sql`, `005_collaboration_annotations.sql`, and `006_health_timeline.sql` are applied
- THEN schema changes MUST be additive
- AND rollback/recovery MUST remain possible via documented backup procedure

### Requirement: V3 Scope Protection and Non-Goals

The system SHALL enforce v3 non-goals to prevent scope creep during execution.

#### Scenario: Out-of-scope request appears during v3

- GIVEN a request for cloud multi-tenant realtime sync, full CRDT/distributed conflict resolution, or unrelated v4 capability
- WHEN evaluated for inclusion in `v3-collaboration-platform`
- THEN the request MUST be marked out of scope
- AND planning MUST defer it to a future approved change
