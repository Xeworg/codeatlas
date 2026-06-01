# Delta for Project Understanding MVP

## ADDED Requirements

### Requirement: Architecture Detection with Evidence

The system MUST provide architecture detection results for each analyzed project, including detected pattern, confidence score, and supporting evidence.

#### Scenario: Architecture detection returns classified result

- GIVEN a scanned project with enough structural signals
- WHEN the user requests architecture detection
- THEN the system MUST return one of `mvc`, `layered`, `clean`, `hexagonal`, or `unknown`
- AND the system MUST include a confidence value
- AND the system MUST include evidence traceable to project graph elements

#### Scenario: Detection degrades safely on failure

- GIVEN a project where detection cannot be computed reliably
- WHEN architecture detection fails
- THEN the system MUST return `unknown` with zero confidence
- AND the system MUST NOT crash the analysis flow

### Requirement: Impact Analysis

The system MUST provide impact analysis for a selected node/file and return affected nodes with a bounded impact score.

#### Scenario: Impact result is returned for selected node

- GIVEN a scanned project graph and a selected node
- WHEN the user requests impact analysis
- THEN the system MUST return the selected node identifier
- AND the system MUST return a list of affected nodes
- AND the system MUST return an impact score for the analysis result

### Requirement: Graph Insights (Cycles and Hotspots)

The system MUST compute graph insights including cycles and hotspots for the current project graph.

#### Scenario: Insights are computed successfully

- GIVEN a project graph with dependency relations
- WHEN graph insights are requested
- THEN the system MUST return cycles and hotspots
- AND the system MUST include aggregate graph metrics for coupling and density

#### Scenario: Insights degrade without blocking graph usage

- GIVEN a graph where insight computation times out or fails
- WHEN the failure occurs
- THEN the system MUST return an empty insights payload with explicit failure state
- AND the graph view MUST remain usable

### Requirement: Exportable Analysis Evidence

The system MUST support exporting analysis evidence in JSON and PNG formats for sharing.

#### Scenario: User exports JSON evidence

- GIVEN an analyzed project with graph and insights
- WHEN the user selects JSON export
- THEN the system MUST generate an export payload containing graph data
- AND the payload MUST include insights when available

#### Scenario: PNG export fallback

- GIVEN an analyzed project where PNG export cannot be produced
- WHEN PNG export fails
- THEN the system MUST provide a non-crashing fallback path to JSON export
- AND the user MUST receive a clear warning message

### Requirement: v2 Analytical Views and Persistent Filters

The system MUST provide analytical views for architecture, dependencies, and application flow (beta), and MUST persist filter selections across the active session.

#### Scenario: User switches analytical views

- GIVEN an analyzed project
- WHEN the user switches between architecture, dependencies, and flow views
- THEN each view MUST render the corresponding analysis perspective without re-scanning the project

#### Scenario: Filters persist during session

- GIVEN user-defined filters/groupings in analytical view
- WHEN the user navigates between explorer, graph, and details panels
- THEN the selected filters MUST remain applied during the session

### Requirement: v2 Contract Compatibility

The system MUST expose v2 analysis contracts (`ArchitectureDetectionResult`, `ImpactAnalysisResult`, `GraphInsights`, `ExportPayload`) without breaking v1 consumers.

#### Scenario: v2 contract fields are available

- GIVEN a v2-capable frontend client
- WHEN it invokes analysis commands
- THEN the returned payloads MUST match the declared v2 contract fields

#### Scenario: v1 compatibility is preserved

- GIVEN an existing v1 workflow (scan, graph, details, search)
- WHEN v2 features are present
- THEN v1 workflows MUST continue functioning without required breaking field renames or removals

### Requirement: Additive v2 Data Migration

The system MUST apply v2 database migration changes additively and preserve existing v1 data.

#### Scenario: Migration updates schema without data loss

- GIVEN a database with v1 project data
- WHEN migration `003_architecture_and_insights.sql` is applied
- THEN new v2 schema elements MUST be added
- AND existing v1 rows in `projects`, `files`, `symbols`, and `imports` MUST remain readable

### Requirement: i18n Foundation for Spanish Catalog

The system MUST externalize UI copy into a Spanish catalog and resolve strings through translation keys, while keeping language runtime fixed to Spanish in v2.

#### Scenario: UI strings resolve from catalog

- GIVEN the v2 UI is rendered
- WHEN user-visible text is shown
- THEN text MUST be resolved through translation keys backed by `locales/es.json`
- AND the UI MUST NOT depend on hardcoded user-facing strings in components for migrated surfaces

#### Scenario: No language switcher in v2

- GIVEN v2 runtime configuration
- WHEN the user navigates the UI
- THEN the active language MUST remain Spanish
- AND the system MUST NOT expose a language selector in v2

### Requirement: v3 Scope Exclusion Enforcement

The system MUST reject v3 collaboration and multi-project capabilities from v2 implementation scope.

#### Scenario: v3 feature proposed during v2

- GIVEN a proposal to add workspaces, snapshots, annotations, or health timeline
- WHEN evaluated for v2 inclusion
- THEN the feature MUST be marked out of scope for v2
- AND implementation planning MUST defer it to the v3 change set
