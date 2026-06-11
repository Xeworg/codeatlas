//! CodeAtlas — Tauri entry point
//! Library run() used by main binary.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
pub mod ipc_error;
pub mod logging;

use commands::AppState;
use std::sync::Mutex;
use tauri::Manager;

pub fn run() {
    // Initialize tracing. In dev builds this installs a per-execution
    // non-blocking file writer at `<repo>/logs/dev-runs/codeatlas-dev-*.log`
    // in addition to the existing stderr writer, so the agent/developer can
    // inspect a readable execution log after a run. The dev default level is
    // DEBUG unless `RUST_LOG` overrides it. Release builds keep the previous
    // console-only INFO-default behavior.
    //
    // The guard (if returned) must live for the whole `run()` lifetime;
    // dropping it flushes and stops the background log writer thread.
    let _log_guard = logging::init_dev_file_logging(&logging::compile_time_repo_root());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");
            std::fs::create_dir_all(&app_dir).ok();

            let db_path = app_dir.join("codeatlas.db");
            let db_pool = engine::db::DbPool::new(db_path.to_str().unwrap_or("codeatlas.db"))
                .expect("Failed to open database");

            db_pool.init_schema().ok();
            let migration_result = db_pool.with_connection(|conn| {
                use engine::db::migrations::run_pending_migrations;
                run_pending_migrations(conn)
            });
            if let Err(e) = migration_result {
                tracing::warn!("Migration warning: {:?}", e);
            }

            let pool = db_pool.clone();
            let app_state = AppState {
                scan_status: std::sync::Arc::new(Mutex::new(engine::models::ScanStatus::Idle)),
                ai_config: std::sync::Arc::new(Mutex::new(None)),
                project_root: std::sync::Arc::new(Mutex::new(String::new())),
                ai_service_port: std::sync::Arc::new(engine::ai::AIService::default())
                    as std::sync::Arc<dyn engine::ai::AIServicePort>,
                scan_repo: std::sync::Arc::new(engine::ports::ScanRepositoryAdapter::from_arc(
                    std::sync::Arc::new(pool.clone()),
                )) as std::sync::Arc<dyn engine::ports::ScanRepository>,
                graph_repo: std::sync::Arc::new(engine::ports::GraphRepositoryAdapter::from_arc(
                    std::sync::Arc::new(pool.clone()),
                ))
                    as std::sync::Arc<dyn engine::ports::GraphRepository>,
                analysis_repo: std::sync::Arc::new(
                    engine::ports::AnalysisDataSourceAdapter::from_arc(std::sync::Arc::new(
                        pool.clone(),
                    )),
                )
                    as std::sync::Arc<dyn engine::ports::AnalysisDataSource>,
                workspace_repo: std::sync::Arc::new(
                    engine::ports::WorkspaceRepositoryAdapter::from_arc(std::sync::Arc::new(
                        pool.clone(),
                    )),
                )
                    as std::sync::Arc<dyn engine::ports::WorkspaceRepository>,
            };

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_project,
            commands::open_project_by_path,
            commands::get_scan_status,
            commands::cancel_scan,
            commands::get_dependencies,
            commands::get_dependents,
            commands::get_graph,
            commands::get_node_details,
            commands::get_node_outline,
            commands::search_nodes,
            commands::configure_ai,
            commands::get_ai_config,
            commands::explain_node,
            commands::chat,
            commands::get_architecture_detection,
            commands::get_impact_analysis,
            commands::get_graph_insights,
            commands::export_view,
            commands::create_workspace,
            commands::list_workspaces,
            commands::attach_project_to_workspace,
            commands::list_workspace_projects,
            commands::create_snapshot,
            commands::get_snapshot,
            commands::list_snapshots,
            commands::add_comment,
            commands::list_comments,
            commands::get_health_timeline,
            commands::get_executive_summary,
            commands::compare_snapshots,
            commands::get_c4_view,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
