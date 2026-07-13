// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod error;
mod state;
mod commands;

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tauri::{Manager, Emitter};
use ll_core::collector::LogCollector;
use ll_core::clustering::ClusterGrouper;
use state::{AppState, AppSettings};
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("loglens=debug".parse().unwrap()))
        .with_target(false)
        .init();

    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            let data_dir = app.path().app_data_dir().unwrap();
            std::fs::create_dir_all(&data_dir).unwrap();
            let db_path = data_dir.join("loglens.db");

            let pool = tauri::async_runtime::block_on(ll_core::db::open(&db_path)).unwrap();

            // Load settings from DB or use defaults
            let settings_json = tauri::async_runtime::block_on(ll_core::db::queries::get_setting(&pool, "app_settings"))
                .unwrap_or(None);
            let settings: AppSettings = settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            let (log_tx, mut log_rx) = mpsc::channel::<ll_core::models::log_entry::NormalizedEntry>(4096);
            let grouper = Arc::new(ClusterGrouper::new());
            let custom_parsers = ll_core::normalizer::CustomParserSet::compile(&settings.custom_parsers);
            let collector = Arc::new(LogCollector::new(log_tx.clone(), grouper.clone(), custom_parsers));

            // Restore previously configured sources
            let saved_sources = tauri::async_runtime::block_on(ll_core::db::queries::list_sources(&pool)).unwrap_or_default();
            for src in saved_sources {
                if src.enabled {
                    let c = collector.clone();
                    let s = src.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = c.watch(s).await {
                            tracing::warn!("Failed to restore source: {}", e);
                        }
                    });
                }
            }

            // Forward log entries to frontend
            let pool2 = pool.clone();
            let grouper2 = grouper.clone();
            let ah = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(entry) = log_rx.recv().await {
                    // Persist
                    if let Err(e) = ll_core::db::queries::insert_entry(&pool2, &entry).await {
                        tracing::warn!("DB insert failed: {}", e);
                    }

                    // Emit to frontend
                    let _ = ah.emit("log://entry", &entry);

                    // Cluster spikes
                    let _ = ah.emit("cluster://updated", grouper2.top_errors(5));
                }
            });

            let state = AppState {
                pool,
                collector,
                grouper,
                settings: Arc::new(RwLock::new(settings)),
            };

            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Settings
            commands::get_settings,
            commands::save_settings,
            commands::save_api_key,
            commands::has_api_key,
            commands::check_ai_backend,
            // Collector
            commands::add_source,
            commands::remove_source,
            commands::list_sources,
            commands::watch_file,
            commands::watch_docker,
            // Query
            commands::query_logs,
            commands::get_timeline,
            commands::translate_query,
            // Clusters
            commands::list_clusters,
            commands::get_cluster_stats,
            // AI
            commands::explain_entry,
            commands::get_cached_explanation,
            commands::summarize_entries,
            commands::analyze_cluster,
        ])
        .run(tauri::generate_context!())
        .expect("error running LogLens");
}
