//! AnalysisService integration tests — RED phase (T14).
//!
//! These tests verify that `AnalysisService` exists and provides the correct
//! orchestration surface for analysis/export commands. Tests fail because
//! `engine::services::AnalysisService` does not yet exist.

use engine::db::DbPool;
use engine::ports::{AnalysisDataSourceAdapter, GraphRepositoryAdapter};
use engine::services::AnalysisService;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn init_schema(pool: &DbPool, project_id: &str) {
    pool.with_connection(|conn| {
        engine::db::schema::init_schema(conn).ok();
        conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, root_path) VALUES (?1, 'Test', '/tmp')",
            rusqlite::params![project_id],
        )
        .ok();
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();
}

fn insert_files(pool: &DbPool, project_id: &str, files: &[(&str, &str)]) {
    pool.with_connection(|conn| {
        for (id, path) in files {
            conn.execute(
                "INSERT OR REPLACE INTO files (id, project_id, path, name, extension)
                 VALUES (?1, ?2, ?3, ?1, 'ts')",
                rusqlite::params![id, project_id, path],
            )
            .ok();
        }
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();
}

/// Creates the graph_insights table (not in init_schema, requires migrations).
fn ensure_graph_insights_table(pool: &DbPool) {
    pool.with_connection(|conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS graph_insights (
                project_id TEXT PRIMARY KEY,
                cycles TEXT NOT NULL,
                hotspots TEXT NOT NULL,
                avg_coupling REAL,
                density REAL,
                generated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            [],
        )
        .ok();
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();
}

fn service(pool: &DbPool) -> AnalysisService<AnalysisDataSourceAdapter, GraphRepositoryAdapter> {
    let analysis_repo = AnalysisDataSourceAdapter::new(pool);
    let graph_repo = GraphRepositoryAdapter::new(pool);
    AnalysisService::new(analysis_repo, graph_repo)
}

// ─────────────────────────────────────────────────────────────────────────────
// T14.1 — get_architecture_detection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn t14_1_get_architecture_detection_returns_pattern_and_confidence() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-arch");
    insert_files(
        &pool,
        "proj-arch",
        &[
            ("f1", "src/domain/User.ts"),
            ("f2", "src/application/UseCase.ts"),
            ("f3", "src/infrastructure/Repo.ts"),
        ],
    );

    let result = service(&pool)
        .get_architecture_detection("proj-arch")
        .expect("AnalysisService should return architecture detection result");

    assert_eq!(result.version, "2.0");
    assert_eq!(result.pattern, "clean");
    assert!(result.confidence > 0.0);
    assert!(!result.generated_at.is_empty());
}

#[test]
fn t14_2_get_architecture_detection_unknown_pattern_when_no_indicators() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-neutral");
    insert_files(
        &pool,
        "proj-neutral",
        &[("f1", "src/index.ts"), ("f2", "src/utils.ts")],
    );

    let result = service(&pool)
        .get_architecture_detection("proj-neutral")
        .expect("AnalysisService should return result even for neutral projects");

    assert_eq!(result.pattern, "unknown");
    assert_eq!(result.confidence, 0.0);
}

#[test]
fn t14_3_get_architecture_detection_persists_result() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-persist");
    insert_files(
        &pool,
        "proj-persist",
        &[("f1", "src/models/User.ts"), ("f2", "src/views/Home.tsx")],
    );

    let r1 = service(&pool)
        .get_architecture_detection("proj-persist")
        .expect("First call should succeed");

    let r2 = service(&pool)
        .get_architecture_detection("proj-persist")
        .expect("Second call should succeed");

    assert_eq!(r1.pattern, r2.pattern);
    assert_eq!(r1.confidence, r2.confidence);
}

// ─────────────────────────────────────────────────────────────────────────────
// T14.4 — get_impact_analysis
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn t14_4_get_impact_analysis_returns_affected_nodes_and_score() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-impact");
    // f0 (A.ts) → f1 (B.ts) → f2 (C.ts): B imports A, C imports B
    insert_files(
        &pool,
        "proj-impact",
        &[("f0", "src/A.ts"), ("f1", "src/B.ts"), ("f2", "src/C.ts")],
    );
    pool.with_connection(|conn| {
        for (src, tgt) in [("f1", "f0"), ("f2", "f1")] {
            conn.execute(
                "INSERT OR IGNORE INTO imports (id, source_file_id, target_file_id, target_module, import_names)
                 VALUES (?1, ?2, ?3, '', 'default')",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), src, tgt],
            )
            .ok();
        }
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();

    let result = service(&pool)
        .get_impact_analysis("proj-impact", "f0")
        .expect("AnalysisService should return impact analysis result");

    assert_eq!(result.version, "2.0");
    assert_eq!(result.changed_node_id, "f0");
    assert!(
        !result.affected_nodes.is_empty(),
        "Expected affected nodes for f0 (imported by f1)"
    );
    assert!(
        result.impact_score >= 0.0 && result.impact_score <= 1.0,
        "impact_score {} out of [0,1]",
        result.impact_score
    );
    assert!(!result.explanation.is_empty());
}

#[test]
fn t14_5_get_impact_analysis_isolated_node_returns_empty_affected() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-isolated");
    insert_files(&pool, "proj-isolated", &[("f0", "src/Orphan.ts")]);

    let result = service(&pool)
        .get_impact_analysis("proj-isolated", "f0")
        .expect("AnalysisService should return result for isolated node");

    assert_eq!(result.changed_node_id, "f0");
    assert!(
        result.affected_nodes.is_empty(),
        "Expected empty affected for isolated node"
    );
    assert!(result.impact_score >= 0.0);
}

// ─────────────────────────────────────────────────────────────────────────────
// T14.6 — get_graph_insights
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn t14_6_get_graph_insights_returns_cycles_hotspots_density() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-insights");
    // f0 ← f1 ← f0 (cycle): f1 imports f0, f0 imports f1
    insert_files(
        &pool,
        "proj-insights",
        &[("f0", "src/A.ts"), ("f1", "src/B.ts")],
    );
    pool.with_connection(|conn| {
        for (src, tgt) in [("f1", "f0"), ("f0", "f1")] {
            conn.execute(
                "INSERT OR IGNORE INTO imports (id, source_file_id, target_file_id, target_module, import_names)
                 VALUES (?1, ?2, ?3, '', 'default')",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), src, tgt],
            )
            .ok();
        }
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();

    let result = service(&pool)
        .get_graph_insights("proj-insights")
        .expect("AnalysisService should return graph insights result");

    assert_eq!(result.version, "2.0");
    assert!(
        !result.cycles.is_empty(),
        "Expected at least 1 cycle for A↔B circular import"
    );
    assert!(result.density.is_some(), "Expected density to be computed");
    assert!(
        result.avg_coupling.is_some(),
        "Expected avg_coupling to be computed"
    );
}

#[test]
fn t14_7_get_graph_insights_persists_result() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-insights-persist");
    insert_files(
        &pool,
        "proj-insights-persist",
        &[("f0", "src/A.ts"), ("f1", "src/B.ts")],
    );
    pool.with_connection(|conn| {
        for (src, tgt) in [("f1", "f0"), ("f0", "f1")] {
            conn.execute(
                "INSERT OR IGNORE INTO imports (id, source_file_id, target_file_id, target_module, import_names)
                 VALUES (?1, ?2, ?3, '', 'default')",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), src, tgt],
            )
            .ok();
        }
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();

    let r1 = service(&pool)
        .get_graph_insights("proj-insights-persist")
        .expect("First call should succeed");

    let r2 = service(&pool)
        .get_graph_insights("proj-insights-persist")
        .expect("Second call should succeed");

    assert_eq!(r1.cycles.len(), r2.cycles.len());
    assert_eq!(r1.hotspots.len(), r2.hotspots.len());
}

// ─────────────────────────────────────────────────────────────────────────────
// T14.8 — export_view
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn t14_8_export_view_json_returns_valid_payload() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-export");
    ensure_graph_insights_table(&pool);
    insert_files(&pool, "proj-export", &[("f0", "src/A.ts")]);

    // Pre-cache a graph for the project
    pool.with_connection(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO project_cache (project_id, graph_json, generated_at)
             VALUES ('proj-export', '{\"nodes\":[{\"id\":\"f0\",\"label\":\"A.ts\"}],\"edges\":[]}', '2026-01-01T00:00:00Z')",
            [],
        )
        .ok();
        Ok::<(), rusqlite::Error>(())
    })
    .unwrap();

    let result = service(&pool)
        .export_view("proj-export", String::from("json"))
        .expect("AnalysisService should return export payload for json format");

    assert_eq!(result.version, "2.0");
    assert_eq!(result.format, "json");
    assert!(result.graph_data.is_object());
    assert_eq!(result.metadata.project_id, "proj-export");
    assert!(!result.metadata.generated_at.is_empty());
}

#[test]
fn t14_9_export_view_png_returns_png_error() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-export-png");

    let result = service(&pool).export_view("proj-export-png", String::from("png"));

    assert!(
        result.is_err(),
        "Expected error for png format (frontend responsibility)"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("png") || err_msg.contains("frontend"),
        "Error message should indicate png is frontend responsibility"
    );
}

#[test]
fn t14_10_export_view_invalid_format_returns_error() {
    let pool = DbPool::in_memory().unwrap();
    init_schema(&pool, "proj-export-invalid");

    let result = service(&pool).export_view("proj-export-invalid", String::from("svg"));

    assert!(
        result.is_err(),
        "Expected error for unsupported format 'svg'"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid export format") || err_msg.contains("Supported"),
        "Error message should mention supported formats"
    );
}
