use serde::Serialize;
use std::sync::Arc;
use tauri::Manager;
mod accounts;
mod actions;
mod adapters;
mod ai_doctor;
mod app;
mod app_preferences;
mod backup;
mod config;
mod core;
mod diagnostics;
mod discovery;
mod domain;
mod health;
mod ipc;
#[cfg(test)]
mod keychain_selftest;
mod lifecycle;
mod models;
mod native_confirm;
mod network;
mod platform;
#[cfg(test)]
mod scenario_tests;
mod secrets;
mod sessions;
mod store;
mod sync;
mod usage_insights;
mod workspaces;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    contract_version: u8,
    version: &'static str,
    preferred_locale: Option<String>,
}

// No inputs and no domain observations. ACL admits only the bundled main UI.
#[tauri::command]
fn app_metadata() -> AppMetadata {
    AppMetadata {
        contract_version: 1,
        version: env!("CARGO_PKG_VERSION"),
        preferred_locale: sys_locale::get_locale(),
    }
}

pub fn run() {
    tauri::Builder::default()
        .menu(|app| app_preferences::create_menu(app, app_preferences::LocalePreference::English))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            app_preferences::apply_app_preferences,
            ipc::nexus_overview,
            ipc::nexus_agent_history,
            ipc::nexus_models,
            ipc::nexus_create_provider,
            ipc::nexus_set_provider_secret,
            ipc::nexus_delete_provider,
            ipc::nexus_check_provider,
            ipc::nexus_create_model,
            ipc::nexus_delete_model,
            ipc::nexus_save_profile,
            ipc::nexus_delete_profile,
            ipc::nexus_set_assignment,
            ipc::nexus_prepare_action,
            ipc::nexus_confirm_plan,
            ipc::nexus_discard_plan,
            ipc::nexus_launch_agent,
            ipc::nexus_storage_status,
            ipc::nexus_prune_history,
            ipc::nexus_diagnostic_preview,
            ipc::nexus_list_workspaces,
            ipc::nexus_create_workspace,
            ipc::nexus_delete_workspace,
            ipc::nexus_launch_workspace,
            ipc::nexus_consult_ai_doctor,
            ipc::nexus_list_sessions,
            ipc::nexus_resume_session,
            ipc::nexus_delete_session,
            ipc::nexus_usage_insights,
            ipc::nexus_get_account,
            ipc::nexus_switch_account_tier,
            ipc::nexus_list_sync_devices,
            ipc::nexus_trigger_sync,
            ipc::nexus_resolve_sync_conflict,
            ipc::nexus_export_account_bundle,
            ipc::nexus_purge_cloud_sync
        ])
        .setup(|app| {
            // Nexus-owned data lives in the per-app data directory, never inside Agent folders.
            let data_dir = app.path().app_data_dir()?;
            let ctx = platform::HostContext::detect(data_dir.clone());
            platform::fs::ensure_private_dir(&data_dir).map_err(|e| e.to_string())?;
            let store =
                store::Store::open(&data_dir.join("nexus.db")).map_err(|e| e.to_string())?;
            let confirmer = Box::new(native_confirm::NativeConfirmer {
                handle: app.handle().clone(),
            });
            let state = app::App::new(ctx, store, Arc::new(secrets::OsSecretStore), confirmer)
                .map_err(|e| e.to_string())?;
            app.manage::<ipc::AppState>(Arc::new(state));
            if let Some(window) = app.get_webview_window("main") {
                // Never force the practical minimum beyond the monitor work area.
                if let Some(monitor) = window.current_monitor()? {
                    let area = monitor.work_area();
                    let size = area.size.to_logical::<f64>(monitor.scale_factor());
                    let width = size.width.min(1200.0);
                    let height = (size.height - 40.0).clamp(1.0, 800.0);
                    window.set_min_size(Some(tauri::LogicalSize::new(
                        width.min(960.0),
                        height.min(640.0),
                    )))?;
                    window.set_size(tauri::LogicalSize::new(width, height))?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Project Nexus desktop runtime failed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn metadata_is_versioned_and_contains_only_application_facts() {
        let metadata = super::app_metadata();
        assert_eq!(metadata.contract_version, 1);
        assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
        if let Some(locale) = metadata.preferred_locale {
            assert!(!locale.is_empty());
        }
    }
}
