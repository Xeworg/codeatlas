# Changelog de Contratos API

## v1.1 (planificado — post-MVP)
- `scan_project`: agregar campo opcional `exclude_patterns?: string[]`
- `GraphNode`: agregar campo opcional `group?: string`
- `chat`: agregar campo opcional `session_id?: string`
- Nuevos comandos: `list_projects`, `delete_project`

## v2.0 (planificado)
- `get_architecture_detection(projectId) → ArchitectureDetectionResult`
- `get_impact_analysis(nodeId, direction) → ImpactAnalysisResult`
- `get_graph_insights(projectId) → GraphInsights`
- `GraphData`: agregar campo opcional `insights?: GraphInsights`
- `export_view(projectId, format) → binary`
- Nuevo `NodeType`: `middleware`, `guard`, `interceptor`

## v3.0 (planificado)
- `create_snapshot(projectId, label) → Snapshot`
- `add_comment(nodeId, text, author) → Comment`
- `share_view(viewId, recipients) → ShareLink`
- `get_health_timeline(projectId, from, to) → HealthScoreTimeline`
- `get_executive_summary(workspaceId) → ExecutiveArchitectureSummary`

## v1.0 (MVP — actual)
- `scan_project(path) → ScanResult`
- `get_scan_status(projectId) → ScanStatus`
- `cancel_scan(projectId) → void`
- `get_graph(projectId) → GraphData`
- `get_node_details(nodeId) → NodeDetails`
- `search_nodes(projectId, query, limit) → SearchResult`
- `get_dependencies(nodeId) → GraphEdge[]`
- `get_dependents(nodeId) → GraphEdge[]`
- `explain_node(nodeId, symbolId?) → NodeExplanation`
- `chat(projectId, message, history, contextNodeIds?) → ChatResponse`
- `configure_ai(config) → void`
- `get_ai_config() → AIConfig`
