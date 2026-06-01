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

    let state: Mutex<Option<AppState>> = Mutex::new(None);

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
            let db_pool = engine::db::DbPool::new(
                db_path.to_str().unwrap_or("codeatlas.db"),
            )
            .expect("Failed to open database");

            db_pool.init_schema().ok();

            let app_state = AppState {
                db: db_pool,
                scan_status: Mutex::new(commands::ScanStatus::Idle),
                ai_config: Mutex::new(None),
                project_root: String::new(),
            };

            app.manage(app_state);

            let mut global_state = state.lock().unwrap();
            *global_state = Some(app_state);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
