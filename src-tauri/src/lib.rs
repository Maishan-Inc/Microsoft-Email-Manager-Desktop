mod accounts;
mod accounts_auth;
mod background;
mod commands;
mod crypto;
mod db;
mod error;
mod export;
mod graph;
mod imap_client;
mod models;
mod state;

use std::path::PathBuf;

use tauri::Manager;

use state::AppState;

/// 计算数据库路径：放在系统应用数据目录下。
fn resolve_db_path(app: &tauri::App) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    std::fs::create_dir_all(&dir).ok();
    dir.join("mem-desktop.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let db_path = resolve_db_path(app);
            log::info!("数据库路径: {}", db_path.display());
            app.manage(AppState::new(db_path));
            background::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::setup_master_password,
            commands::unlock,
            commands::lock,
            commands::list_accounts,
            commands::add_account,
            commands::delete_account,
            commands::update_classification,
            commands::import_accounts,
            commands::test_credentials,
            commands::test_account,
            commands::get_catalog,
            commands::add_category,
            commands::add_tag,
            commands::delete_category,
            commands::delete_tag,
            commands::list_emails,
            commands::get_email_details,
            commands::export_accounts,
            commands::check_account_health,
            commands::set_account_notify,
            commands::dashboard_stats,
            commands::sync_mail_now,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
