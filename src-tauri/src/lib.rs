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
use std::collections::HashSet;
use std::time::Duration;
use crate::database::Database;
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
            // 產生 ISO 8601 格式的本地時間戳記
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs();
            // 簡易 UTC 時間格式化（不依賴外部套件）
            let (y, mo, d, h, mi, s) = {
                let mut t = secs;
                let ss = t % 60; t /= 60;
                let mm = t % 60; t /= 60;
                let hh = t % 24; t /= 24;
                // 自 1970-01-01 起算日期
                let mut days = t;
                let mut year = 1970u64;
                loop {
                    let dy = if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) { 366 } else { 365 };
                    if days < dy { break; }
                    days -= dy; year += 1;
                }
                let is_leap = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
                let months = [31u64,if is_leap{29}else{28},31,30,31,30,31,31,30,31,30,31];
                let mut month = 0usize;
                for &m in &months { if days < m { break; } days -= m; month += 1; }
                (year, month + 1, days + 1, hh, mm, ss)
            };
            let timestamp = format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, mi, s);
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }};
}

pub struct AppState {
    pub db: Database,
    pub watch_paths: Mutex<HashSet<String>>,
    pub event_tx: tokio::sync::broadcast::Sender<String>,
    pub db_switch_count: std::sync::atomic::AtomicUsize,
}

pub async fn run() {
    app_log!("[INIT] 開始執行 run(), 讀取環境變數...");

    let app_data_dir = std::env::var("STICKPLAY_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());
    app_log!("[INIT] STICKPLAY_CONFIG_DIR = {}", app_data_dir);

    let config_path = std::path::PathBuf::from(app_data_dir);
    
    app_log!("[INIT] 正在初始化資料庫...");
    let db = Database::new(config_path.clone()).expect("資料庫初始化失敗");
    app_log!("[INIT] 資料庫初始化完成！");

    let (event_tx, _) = tokio::sync::broadcast::channel::<String>(16);

    let shared_state = Arc::new(AppState {
        db,
        watch_paths: Mutex::new(HashSet::new()),
        event_tx: event_tx.clone(),
        db_switch_count: std::sync::atomic::AtomicUsize::new(0),
    });

    // --- 輕量級目錄輪詢任務 ---
    // 取代 inotify（在 Docker volume mount 環境下無效）
    // 每 30 秒只做 readdir() 比對頂層目錄，I/O 負擔極低
    let poll_state = Arc::clone(&shared_state);
    tokio::spawn(async move {
        app_log!("[POLL] 目錄輪詢任務已啟動（間隔 30 秒）");

        const VIDEO_EXTS: &[&str] = &["mp4", "mkv", "avi", "wmv", "mov", "ts", "flv", "rmvb"];
        const MIN_VIDEO_SIZE: u64 = 300 * 1024 * 1024; // 300MB

        // 記錄已知的資料夾集合（用於比對新增/刪除）
        let mut known_dirs: HashSet<String> = HashSet::new();
        // 首次啟動時先等待 sync_watch_paths 被前端呼叫
        let mut initialized = false;
        let mut last_db_switch = 0;

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            // 檢查資料庫是否已切換
            let current_db_switch = poll_state.db_switch_count.load(std::sync::atomic::Ordering::SeqCst);
            if current_db_switch != last_db_switch {
                app_log!("[POLL] 偵測到資料庫切換，重設已知目錄清單");
                known_dirs.clear();
                initialized = false;
                last_db_switch = current_db_switch;
            }

            let watch_paths: Vec<String> = match poll_state.watch_paths.lock() {
                Ok(guard) => guard.iter().cloned().collect(),
                Err(poisoned) => poisoned.into_inner().iter().cloned().collect(),
            };
            if watch_paths.is_empty() {
                continue;
            }

            // 首次有 watch_paths 時，從 DB 載入已知的資料夾
            if !initialized {
                if let Ok(conn) = poll_state.db.conn.lock() {
                    if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT folder_path FROM videos") {
                        if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                            for row in rows.flatten() {
                                known_dirs.insert(row);
                            }
                        }
                    }
                }
                app_log!("[POLL] 已從 DB 載入 {} 個已知資料夾", known_dirs.len());
                initialized = true;
            }

            // 掃描各監控根目錄下的子資料夾
            let mut current_dirs: HashSet<String> = HashSet::new();
            for root in &watch_paths {
                let root_path = std::path::Path::new(root);
                if !root_path.exists() || !root_path.is_dir() {
                    continue;
                }
                if let Ok(entries) = std::fs::read_dir(root_path) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            current_dirs.insert(entry.path().to_string_lossy().to_string());
                        }
                    }
                }
            }

            // 偵測新增的資料夾
            let added: Vec<String> = current_dirs.difference(&known_dirs).cloned().collect();
            // 偵測刪除的資料夾
            let removed: Vec<String> = known_dirs.difference(&current_dirs)
                .filter(|d| {
                    // 只處理屬於當前 watch_paths 下的資料夾
                    watch_paths.iter().any(|wp| d.starts_with(wp))
                })
                .cloned()
                .collect();

            let mut changed = false;

            // 處理新增
            for dir in &added {
                let dir_path = std::path::Path::new(dir);

                // 檢查是否含有 >300MB 的影片檔
                let has_large_video = std::fs::read_dir(dir_path)
                    .map(|entries| {
                        entries.flatten().any(|e| {
                            let p = e.path();
                            if !p.is_file() { return false; }
                            let is_video = p.extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| VIDEO_EXTS.contains(&ext.to_lowercase().as_str()))
                                .unwrap_or(false);
                            if !is_video { return false; }
                            p.metadata().map(|m| m.len() > MIN_VIDEO_SIZE).unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);

                if has_large_video {
                    app_log!("[POLL] 偵測到新資料夾（含 >300MB 影片）: {}", dir);
                    // 使用 spawn_blocking + catch_unwind 防止圖片處理 panic 導致伺服器崩潰
                    let db_ref = &poll_state.db;
                    let scan_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        crate::scanner::scan_single_folder(db_ref, dir_path, false)
                    }));
                    match scan_result {
                        Ok(Ok(())) => { changed = true; }
                        Ok(Err(e)) => { app_log!("[POLL] 掃描失敗: {}", e); }
                        Err(_) => { app_log!("[POLL] 掃描時發生 panic（已攔截），路徑: {}", dir); }
                    }
                }
            }

            // 處理刪除
            for dir in &removed {
                app_log!("[POLL] 偵測到資料夾移除: {}", dir);
                if let Ok(conn) = poll_state.db.conn.lock() {
                    let _ = conn.execute(
                        "DELETE FROM videos WHERE folder_path = ?1",
                        rusqlite::params![dir],
                    );
                }
                changed = true;
            }

            if changed {
                app_log!("[POLL] 媒體庫已變更，通知前端更新");
                let _ = poll_state.event_tx.send("library_updated".to_string());
            }

            // 更新已知資料夾集合
            known_dirs = current_dirs;
        }
    });

    let frontend_dir = std::env::var("STICKPLAY_FRONTEND_DIR").unwrap_or_else(|_| "../dist".to_string());

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/api/scan_library", post(api::scan_library))
        .route("/api/sync_watch_paths", post(api::sync_watch_paths))
        .route("/api/events", get(api::events))
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
        .route("/api/move_video_folder", post(api::move_video_folder))
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
