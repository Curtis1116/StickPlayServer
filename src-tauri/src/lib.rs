pub mod api;
pub mod auth;
pub mod database;
pub mod libraries;
pub mod models;
pub mod parser;
pub mod player_tools;
pub mod scanner;
pub mod security;

use axum::{
    routing::{get, post},
    Router,
};
use database::Database;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::services::{ServeDir, ServeFile};

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
    pub library_id: String,
    pub watch_paths: Mutex<HashSet<String>>,
    pub event_tx: tokio::sync::broadcast::Sender<String>,
    pub operation: tokio::sync::Mutex<()>,
}
pub struct ServerState {
    pub config: PathBuf,
    pub auth: auth::AuthStore,
    pub libraries: Mutex<HashMap<String, Arc<AppState>>>,
    pub gate: tokio::sync::RwLock<()>,
}

pub fn router(state: Arc<ServerState>, frontend: &str) -> Router {
    let scoped = Router::new()
        .route("/scan_library", post(api::scan_library))
        .route("/sync_watch_paths", post(api::sync_watch_paths))
        .route("/events", get(api::events))
        .route("/rescan_single_video", post(api::rescan_single_video))
        .route("/query_videos", post(api::query_videos))
        .route("/update_video_info", post(api::update_video_info))
        .route("/update_rating", post(api::update_rating))
        .route("/toggle_favorite", post(api::toggle_favorite))
        .route("/get_all_genres", post(api::get_all_genres))
        .route("/get_all_levels", post(api::get_all_levels))
        .route("/get_stats", post(api::get_stats))
        .route("/get_fanart_path", post(api::get_fanart_path))
        .route("/get_folder_images", post(api::get_folder_images))
        .route("/crop_and_save_poster", post(api::crop_and_save_poster))
        .route("/move_video_folder", post(api::move_video_folder))
        .route("/video", get(api::serve_video_file))
        .route("/image", get(api::serve_image_file))
        .route("/playback-tickets", post(libraries::playback_ticket))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            libraries::context,
        ));
    let api = scoped
        .route("/auth/status", get(auth::status))
        .route("/auth/setup", post(auth::setup))
        .route("/auth/login", post(auth::login))
        .route("/auth/recover", post(auth::recover))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/devices", get(auth::devices))
        .route("/auth/revoke", post(auth::revoke))
        .route("/auth/password", post(auth::change_password))
        .route("/player-tools/:platform", get(player_tools::download))
        .route("/playback", get(libraries::playback))
        .route("/switch_database", post(libraries::select))
        .route("/delete_database", post(libraries::delete))
        .route("/list_dirs", post(api::list_dirs))
        .route("/get_libraries", post(libraries::list))
        .route("/save_libraries", post(libraries::save))
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::guard,
        ));
    Router::new()
        .nest("/api", api)
        .fallback_service(
            ServeDir::new(frontend).fallback(ServeFile::new(format!("{frontend}/index.html"))),
        )
        .with_state(state)
}

pub async fn run() {
    let config =
        PathBuf::from(std::env::var("STICKPLAY_CONFIG_DIR").unwrap_or_else(|_| "./config".into()));
    std::fs::create_dir_all(&config).expect("無法建立設定目錄");
    let auth = auth::AuthStore::new(&config).expect("身份驗證初始化失敗");
    let state = Arc::new(ServerState {
        config,
        auth,
        libraries: Mutex::new(HashMap::new()),
        gate: tokio::sync::RwLock::new(()),
    });
    libraries::initialize(&state).expect("媒體庫設定初始化失敗");
    libraries::start_polling(state.clone());
    let frontend = std::env::var("STICKPLAY_FRONTEND_DIR").unwrap_or_else(|_| "../dist".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8099".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("無法綁定 port");
    println!("StickPlay 已啟動；網站來源：{}", state.auth.origin);
    axum::serve(
        listener,
        router(state, &frontend).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("伺服器執行錯誤");
}
