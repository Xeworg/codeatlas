//! CodeAtlas — Tauri entry point
//! Registers commands and starts the application.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use codeatlas_lib::commands;

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
