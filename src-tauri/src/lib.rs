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
    let (tx, mut rx) = mpsc::channel::<String>(100);
    let state_for_watcher = Arc::clone(&shared_state);
    
    // Watcher thread (Consumer)
    tokio::spawn(async move {
        app_log!("Watcher: Background task (consumer) started");

        loop {
            let first_path = match rx.recv().await {
                Some(p) => p,
                None => break,
            };

            let process_path = |p_str: &str| -> Option<std::path::PathBuf> {
                let path = std::path::Path::new(p_str);
                
                // 白名單策略：只允許已知影片副檔名觸發 Watcher
                // 這樣可以確保裁切海報、更新 NFO 等操作不會誤觸重新整理
                const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "avi", "wmv", "mov", "ts", "flv", "rmvb"];
                
                let ext_opt = path.extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase());
                
                let is_video_ext = ext_opt.as_deref().map(|ext| VIDEO_EXTS.contains(&ext)).unwrap_or(false);
                
                let mut should_trigger = false;
                
                if is_video_ext {
                    if path.exists() && path.is_file() {
                        // 存在的影片：確認檔案夠大才觸發（避免空檔案或剛建立的小檔）
                        if let Ok(meta) = std::fs::metadata(path) {
                            if meta.len() > 300 * 1024 * 1024 {
                                should_trigger = true;
                            }
                        }
                    } else if !path.exists() {
                        // 影片被刪除
                        should_trigger = true;
                    }
                }
                // 影像、字幕、NFO、資料夾或任何非影片副檔名（含無副檔名的暫存區塊）
                // 一律不觸發，預設 should_trigger = false
                
                if should_trigger {
                    let dir = if !path.exists() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) {
                            if ["mp4", "mkv", "avi", "wmv", "mov", "ts", "flv", "rmvb"].contains(&ext.as_str()) {
                                path.parent().unwrap_or(path).to_path_buf()
                            } else {
                                path.to_path_buf() 
                            }
                        } else {
                            path.to_path_buf()
                        }
                    } else if path.is_dir() {
                        path.to_path_buf()
                    } else {
                        path.parent().unwrap_or(path).to_path_buf()
                    };
                    Some(dir)
                } else {
                    None
                }
            };
            
            let mut pending_dirs = std::collections::HashSet::new();
            if let Some(d) = process_path(&first_path) {
                pending_dirs.insert(d);
            }

            let timeout_duration = Duration::from_secs(5);
            let start = std::time::Instant::now();
            
            while let Some(remaining) = timeout_duration.checked_sub(start.elapsed()) {
                if let Ok(Some(p)) = tokio::time::timeout(remaining, rx.recv()).await {
                    if let Some(d) = process_path(&p) {
                        pending_dirs.insert(d);
                    }
                } else {
                    break;
                }
            }

            if pending_dirs.is_empty() {
                continue;
            }
            
            tokio::time::sleep(Duration::from_millis(1000)).await;

            let watch_paths = state_for_watcher.watch_paths.lock().unwrap().iter().cloned().collect::<Vec<_>>();
            
            for dir_to_scan in pending_dirs {
                let is_valid = watch_paths.iter().any(|p| dir_to_scan.starts_with(std::path::Path::new(p)));
                if is_valid {
                    if !dir_to_scan.exists() {
                        app_log!("Watcher: Folder/Video deleted, removing from DB: {:?}", dir_to_scan);
                        let conn = state_for_watcher.db.conn.lock().unwrap();
                        let dir_str = dir_to_scan.to_string_lossy().to_string();
                        let _ = conn.execute(
                            "DELETE FROM videos WHERE folder_path = ?1",
                            rusqlite::params![dir_str],
                        );
                    } else if dir_to_scan.is_dir() {
                        let is_root = watch_paths.iter().any(|p| std::path::Path::new(p) == dir_to_scan.as_path());
                        if is_root {
                            app_log!("Watcher: Root modified, scanning all: {:?}", dir_to_scan);
                            let _ = scan_library_paths(&state_for_watcher.db, &watch_paths);
                        } else {
                            app_log!("Watcher: Rescanning single folder: {:?}", dir_to_scan);
                            if let Err(e) = crate::scanner::scan_single_folder(&state_for_watcher.db, &dir_to_scan, false) {
                                if e == "資料夾內無影片檔" {
                                    app_log!("Watcher: No video found in folder, removing from DB: {:?}", dir_to_scan);
                                    let conn = state_for_watcher.db.conn.lock().unwrap();
                                    let dir_str = dir_to_scan.to_string_lossy().to_string();
                                    let _ = conn.execute(
                                        "DELETE FROM videos WHERE folder_path = ?1",
                                        rusqlite::params![dir_str],
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Producer (Notify Adapter)
    let tx_notify = tx.clone();
    let watcher = notify::RecommendedWatcher::new(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in &event.paths {
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
