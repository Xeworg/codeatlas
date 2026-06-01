//! CodeAtlas — Tauri entry point
//! Registers commands and starts the application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use codeatlas_lib::commands::{self, AppState};
use std::sync::Mutex;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

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
            // Apply any pending v2 migrations on every startup (idempotent).
            let migration_result = db_pool.with_connection(|conn| {
                use codeatlas_lib::db::migrations::run_pending_migrations;
                run_pending_migrations(conn)
            });
            if let Err(e) = migration_result {
                tracing::warn!("Migration warning: {:?}", e);
            }

            let app_state = AppState {
                db: db_pool,
                scan_status: Mutex::new(commands::ScanStatus::Idle),
                ai_config: Mutex::new(None),
                project_root: Mutex::new(String::new()),
            };

            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan_project,
            commands::get_scan_status,
            commands::get_graph,
            commands::get_node_details,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
