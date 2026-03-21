pub mod api;
pub mod database;
pub mod models;
pub mod parser;
pub mod scanner;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use notify::{Watcher, Config, EventKind};
use std::collections::HashSet;
use tokio::sync::mpsc;
use std::time::Duration;
use crate::database::Database;
use crate::scanner::scan_library_paths;
use std::sync::Mutex;


#[macro_export]
macro_rules! app_log {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let app_data_dir = std::env::var("STICKPLAY_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());
        std::fs::create_dir_all(&app_data_dir).ok();
        let log_path = std::path::Path::new(&app_data_dir).join("stickplay_server.log");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            use std::io::Write;
            let _ = writeln!(file, "{}", msg);
        }
    }};
}

pub struct AppState {
    pub db: Database,
    pub watch_paths: Mutex<HashSet<String>>,
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

pub async fn run() {
    app_log!("[INIT] 開始執行 run(), 讀取環境變數...");

    let app_data_dir = std::env::var("STICKPLAY_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());
    app_log!("[INIT] STICKPLAY_CONFIG_DIR = {}", app_data_dir);

    let config_path = std::path::PathBuf::from(app_data_dir);
    
    app_log!("[INIT] 正在初始化資料庫...");
    let db = Database::new(config_path.clone()).expect("資料庫初始化失敗");
    app_log!("[INIT] 資料庫初始化完成！");

    let shared_state = Arc::new(AppState {
        db,
        watch_paths: Mutex::new(HashSet::new()),
        watcher: Mutex::new(None),
    });

    // --- Background Watcher Setup ---
    let (tx, mut rx) = mpsc::channel(100);
    let state_for_watcher = Arc::clone(&shared_state);
    
    // Watcher thread (Consumer)
    tokio::spawn(async move {
        app_log!("Watcher: Background task (consumer) started");
        let mut last_trigger = std::time::Instant::now();
        let debounce_duration = Duration::from_secs(5);

        while let Some(path_to_scan) = rx.recv().await {
            // Debounce check
            if last_trigger.elapsed() < debounce_duration {
                continue;
            }
            
            // Wait a bit to ensure file operations are finished
            tokio::time::sleep(Duration::from_millis(1000)).await;
            
            app_log!("Watcher: Detected change in {}, triggering rescan", path_to_scan);
            let paths = state_for_watcher.watch_paths.lock().unwrap().iter().cloned().collect::<Vec<_>>();
            if !paths.is_empty() {
                let _ = scan_library_paths(&state_for_watcher.db, &paths);
            }
            last_trigger = std::time::Instant::now();
        }
    });

    // Producer (Notify Adapter)
    let tx_notify = tx.clone();
    let watcher = notify::RecommendedWatcher::new(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    if let Some(path) = event.paths.first() {
                        let _ = tx_notify.blocking_send(path.to_string_lossy().to_string());
                    }
                }
                _ => {}
            }
        }
    }, Config::default()).expect("無法建立檔案監視器");

    // Store watcher in state
    {
        let mut w_guard = shared_state.watcher.lock().unwrap();
        *w_guard = Some(watcher);
    }

    let frontend_dir = std::env::var("STICKPLAY_FRONTEND_DIR").unwrap_or_else(|_| "../dist".to_string());

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/api/scan_library", post(api::scan_library))
        .route("/api/sync_watch_paths", post(api::sync_watch_paths))
        .route("/api/rescan_single_video", post(api::rescan_single_video))
        .route("/api/query_videos", post(api::query_videos))
        .route("/api/update_video_info", post(api::update_video_info))
        .route("/api/update_rating", post(api::update_rating))
        .route("/api/toggle_favorite", post(api::toggle_favorite))
        .route("/api/get_all_genres", post(api::get_all_genres))
        .route("/api/get_all_levels", post(api::get_all_levels))
        .route("/api/get_stats", post(api::get_stats))
        .route("/api/switch_database", post(api::switch_database))
        .route("/api/delete_database", post(api::delete_database))
        .route("/api/get_fanart_path", post(api::get_fanart_path))
        .route("/api/list_dirs", post(api::list_dirs))
        .route("/api/get_libraries", post(api::get_libraries))
        .route("/api/save_libraries", post(api::save_libraries))
        .route("/api/get_folder_images", post(api::get_folder_images))
        .route("/api/crop_and_save_poster", post(api::crop_and_save_poster))
        .route("/api/video", get(api::serve_video_file))
        .route("/api/image", get(api::serve_image_file))
        .fallback_service(ServeDir::new(&frontend_dir).fallback(ServeFile::new(format!("{}/index.html", frontend_dir))))
        .layer(cors)
        .with_state(shared_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8099".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.expect("無法綁定 port");
    app_log!("Server running on http://{}", addr);
    axum::serve(listener, app).await.expect("伺服器執行錯誤");
}
