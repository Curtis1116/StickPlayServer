use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::{IntoResponse, Sse, sse::Event},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tower_http::services::ServeFile;
use tower::ServiceExt;
use futures_util::stream::Stream;

use crate::models::{VideoEntry, VideoFilter, Library};
use crate::parser::{update_nfo, update_nfo_full};
use crate::scanner::scan_single_folder;
use crate::AppState;

type ApiError = (StatusCode, String);
type ApiResult<T> = Result<Json<T>, ApiError>;

fn map_err(e: impl ToString) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPathsPayload {
    pub paths: Vec<String>,
}

pub async fn scan_library(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanPathsPayload>,
) -> ApiResult<usize> {
    crate::scanner::scan_library_paths(&state.db, &payload.paths)
        .map(Json)
        .map_err(map_err)
}

pub async fn sync_watch_paths(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanPathsPayload>,
) -> ApiResult<()> {
    crate::app_log!("API: sync_watch_paths: {:?}", payload.paths);
    
    let mut watch_paths = state.watch_paths.lock().unwrap();
    watch_paths.clear();
    for p in payload.paths {
        if !p.is_empty() {
            watch_paths.insert(p);
        }
    }
    
    Ok(Json(()))
}

/// SSE endpoint：讓前端即時收到媒體庫變更通知
pub async fn events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.event_tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    yield Ok(Event::default().data(msg));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // 跳過遺失的訊息，繼續接收
                    continue;
                }
                Err(_) => break,
            }
        }
    };

    Sse::new(stream)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RescanPayload {
    pub folder_path: String,
}

pub async fn rescan_single_video(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RescanPayload>,
) -> ApiResult<VideoEntry> {
    let dir = std::path::Path::new(&payload.folder_path);
    if !dir.exists() || !dir.is_dir() {
        let conn = state.db.conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM videos WHERE folder_path = ?1",
            rusqlite::params![payload.folder_path],
        );
        return Err(map_err("資料夾不存在，已從資料庫移除"));
    }

    if let Err(e) = scan_single_folder(&state.db, dir, false) {
        if e == "資料夾內無影片檔" {
            let conn = state.db.conn.lock().unwrap();
            let _ = conn.execute(
                "DELETE FROM videos WHERE folder_path = ?1",
                rusqlite::params![payload.folder_path],
            );
            return Err(map_err("影片實體檔案不存在，已從資料庫移除"));
        }
        return Err(map_err(e));
    }

    let filter = VideoFilter {
        search: Some(
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        ),
        genres: None,
        levels: None,
        sort_by: None,
        sort_order: None,
        favorites_only: None,
    };
    let videos = state.db.query_videos(&filter).map_err(map_err)?;

    let entry = videos
        .into_iter()
        .find(|v| v.folder_path == payload.folder_path)
        .ok_or_else(|| map_err("找不到更新後的影片資料"))?;

    Ok(Json(entry))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPayload {
    pub filter: VideoFilter,
}

pub async fn query_videos(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QueryPayload>,
) -> ApiResult<Vec<VideoEntry>> {
    state.db.query_videos(&payload.filter).map(Json).map_err(map_err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFanartPayload {
    pub folder_path: String,
    pub video_path: String,
}

pub async fn get_fanart_path(
    Json(payload): Json<GetFanartPayload>,
) -> ApiResult<String> {
    let folder = Path::new(&payload.folder_path);
    if !folder.exists() || !folder.is_dir() {
        return Err(map_err("資料夾不存在"));
    }

    let video = Path::new(&payload.video_path);
    let video_stem = video
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder_name = folder
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut fanart_path = None;

    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                {
                    if ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "webp" {
                        let file_stem = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();

                        if file_stem.eq_ignore_ascii_case(&video_stem) {
                            return Ok(Json(path.to_string_lossy().to_string()));
                        }

                        if !folder_name.is_empty() && file_stem.eq_ignore_ascii_case(&folder_name) {
                            return Ok(Json(path.to_string_lossy().to_string()));
                        }

                        if file_stem.to_lowercase().contains("fanart") {
                            fanart_path = Some(path.to_string_lossy().to_string());
                        }

                        if fanart_path.is_none() {
                            fanart_path = Some(path.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    fanart_path.map(Json).ok_or_else(|| map_err("找不到縮圖"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVideoInfoPayload {
    pub original_id: String,
    pub video_id: String,
    pub title: String,
    pub level: String,
    pub rating: f64,
    pub criticrating: i32,
    pub actors: Vec<String>,
    pub release_date: String,
    pub date_added: String,
    pub is_favorite: bool,
    pub is_uncensored: bool,
    pub video_path: String,
    pub folder_path: String,
    pub poster_path: Option<String>,
    pub nfo_path: Option<String>,
    pub _nfos_path: Option<String>,
}

pub async fn update_video_info(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateVideoInfoPayload>,
) -> ApiResult<String> {
    if payload.original_id != payload.video_id {
        let conn = state.db.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE videos SET id = ?1 WHERE id = ?2",
            rusqlite::params![payload.video_id, payload.original_id],
        );
        let _ = conn.execute(
            "UPDATE video_actors SET video_id = ?1 WHERE video_id = ?2",
            rusqlite::params![payload.video_id, payload.original_id],
        );
        let _ = conn.execute(
            "UPDATE video_genres SET video_id = ?1 WHERE video_id = ?2",
            rusqlite::params![payload.video_id, payload.original_id],
        );
    }

    let target_nfo = if let Some(ref nfo) = payload.nfo_path {
        nfo.clone()
    } else {
        let folder_p = Path::new(&payload.folder_path);
        // 優先尋找資料夾內已存在的任何 .nfo 檔
        let mut found_nfo = None;
        if let Ok(entries) = std::fs::read_dir(folder_p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e.to_string_lossy().to_lowercase() == "nfo") {
                    found_nfo = Some(path.to_string_lossy().to_string());
                    break;
                }
            }
        }
        
        found_nfo.unwrap_or_else(|| {
            folder_p
                .join(format!("{}.nfo", payload.video_id))
                .to_string_lossy()
                .to_string()
        })
    };

    let nfo_p = Path::new(&target_nfo);
    update_nfo_full(
        nfo_p,
        &payload.video_id,
        payload.rating,
        Some(payload.criticrating),
        &payload.actors,
        &payload.release_date,
        &payload.date_added,
        payload.is_uncensored,
    ).map_err(map_err)?;

    let mut new_genres = Vec::new();
    if let Ok(conn) = state.db.conn.lock() {
        if let Ok(mut stmt) = conn.prepare("SELECT genre FROM video_genres WHERE video_id = ?1") {
            if let Ok(rows) =
                stmt.query_map(rusqlite::params![&payload.video_id], |row| row.get::<_, String>(0))
            {
                new_genres = rows.filter_map(|r| r.ok()).collect();
            }
        }
    }

    new_genres.retain(|g| g != "無碼");
    if payload.is_uncensored {
        new_genres.push("無碼".to_string());
    }

    state.db.upsert_video(
        &payload.video_id,
        &payload.title,
        &payload.level,
        Some(payload.rating),
        &payload.release_date,
        &payload.date_added,
        &payload.video_path,
        &payload.folder_path,
        payload.poster_path.as_deref(),
        Some(&target_nfo),
        None, // 徹底拋棄 nfos_path
        &payload.actors,
        &new_genres,
        payload.criticrating,
    )
    .map_err(map_err)?;

    {
        let conn = state.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET is_favorite = ?1 WHERE id = ?2",
            rusqlite::params![if payload.is_favorite { 1 } else { 0 }, payload.video_id],
        )
        .map_err(map_err)?;
    }

    Ok(Json(target_nfo))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRatingPayload {
    pub video_id: String,
    pub rating: f64,
    pub criticrating: i32,
    pub nfo_path: Option<String>,
    pub _nfos_path: Option<String>,
    pub folder_path: Option<String>,
}

pub async fn update_rating(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRatingPayload>,
) -> ApiResult<String> {
    state.db.update_rating(&payload.video_id, payload.rating, payload.criticrating)
        .map_err(map_err)?;

    let target_nfo = if let Some(ref nfo) = payload.nfo_path {
        nfo.clone()
    } else if let Some(ref folder) = payload.folder_path {
        let folder_p = Path::new(folder);
        let mut found_nfo = None;
        if let Ok(entries) = std::fs::read_dir(folder_p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e.to_string_lossy().to_lowercase() == "nfo") {
                    found_nfo = Some(path.to_string_lossy().to_string());
                    break;
                }
            }
        }

        found_nfo.unwrap_or_else(|| {
            folder_p
                .join(format!("{}.nfo", payload.video_id))
                .to_string_lossy()
                .to_string()
        })
    } else {
        return Ok(Json(String::new()));
    };

    let nfo_p = Path::new(&target_nfo);
    update_nfo(nfo_p, &payload.video_id, payload.rating, Some(payload.criticrating)).map_err(map_err)?;

    {
        let conn = state.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET nfo_path = ?1, nfos_path = NULL WHERE id = ?2",
            rusqlite::params![target_nfo, payload.video_id],
        )
        .map_err(map_err)?;
    }

    Ok(Json(target_nfo))
}

pub async fn get_folder_images(
    Json(payload): Json<RescanPayload>, // reuse the folder_path payload
) -> ApiResult<Vec<String>> {
    let folder = Path::new(&payload.folder_path);
    if !folder.exists() || !folder.is_dir() {
        return Err(map_err("資料夾不存在"));
    }

    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                {
                    if ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "webp" {
                        images.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    images.sort();
    Ok(Json(images))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CropPayload {
    pub video_id: Option<String>,
    pub image_path: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub output_folder: String,
}

pub async fn crop_and_save_poster(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CropPayload>,
) -> ApiResult<String> {
    let state_cloned = Arc::clone(&state);
    let payload_cloned = payload;

    // 將耗時的圖片處理與檔案 I/O 移至 blocking thread
    let result: Result<String, String> = tokio::task::spawn_blocking(move || {
        use image::GenericImageView;

        let img_path = Path::new(&payload_cloned.image_path);
        if !img_path.exists() {
            return Err("來源圖片不存在".to_string());
        }

        let mut img = image::open(img_path).map_err(|e| e.to_string())?;
        
        let (img_w, img_h) = img.dimensions();
        let safe_x = payload_cloned.x.min(img_w);
        let safe_y = payload_cloned.y.min(img_h);
        let safe_w = payload_cloned.width.min(img_w - safe_x);
        let safe_h = payload_cloned.height.min(img_h - safe_y);

        if safe_w == 0 || safe_h == 0 {
            return Err("裁切區域無效".to_string());
        }

        let cropped = img.crop(safe_x, safe_y, safe_w, safe_h);
        
        let out_dir = Path::new(&payload_cloned.output_folder);
        if !out_dir.exists() {
            return Err("輸出資料夾不存在".to_string());
        }

        // 決定目標檔名：優先取代 poster.jpg
        let target_path = out_dir.join("poster.jpg");
        cropped.save(&target_path).map_err(|e| e.to_string())?;

        // 刪除可能存在的 stick_poster.jpg
        let stick_poster_path = out_dir.join("stick_poster.jpg");
        if stick_poster_path.exists() {
            let _ = std::fs::remove_file(&stick_poster_path);
        }

        // 尋找 NFO 路徑：優先依據 video_id 從資料庫查找
        let mut nfo_path_opt = None;
        let mut video_id_final = String::new();

        if let Some(ref vid) = payload_cloned.video_id {
            video_id_final = vid.clone();
            // 從 DB 查找 NFO 路徑
            if let Ok(videos) = state_cloned.db.query_videos(&VideoFilter {
                search: Some(vid.clone()),
                ..Default::default()
            }) {
                if let Some(v) = videos.iter().find(|v| v.id == *vid) {
                    nfo_path_opt = v.nfo_path.clone().map(PathBuf::from);
                }
            }
        }

        // 如果 DB 沒找到或沒給 ID，嘗試在資料夾找第一個 .nfo
        if nfo_path_opt.is_none() {
            if let Ok(entries) = std::fs::read_dir(out_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map_or(false, |e| e.to_string_lossy().to_lowercase() == "nfo") {
                        if let Ok(nfo_data) = crate::parser::parse_nfo(&p) {
                            if video_id_final.is_empty() {
                                video_id_final = nfo_data.num.unwrap_or_default();
                            }
                            nfo_path_opt = Some(p);
                            break;
                        }
                    }
                }
            }
        }

        // 更新縮圖 (Thumbnail)
        if !video_id_final.is_empty() {
            let thumbnail_dir = state_cloned.db.app_data_dir.join("thumbnails");
            let _ = std::fs::create_dir_all(&thumbnail_dir);
            let safe_id = video_id_final.replace("/", "_").replace("\\", "_").replace(":", "_");
            let thumb_path = thumbnail_dir.join(format!("{}.jpg", safe_id));
            let _ = cropped.thumbnail(300, 450).save(&thumb_path);
        }

        // 更新 NFO 標籤 (指向 poster.jpg)
        if let Some(nfo_p) = nfo_path_opt {
             if let Ok(nfo_data) = crate::parser::parse_nfo(&nfo_p) {
                 let _ = crate::parser::update_nfo(
                    &nfo_p,
                    nfo_data.num.as_deref().unwrap_or(""),
                    nfo_data.rating.unwrap_or(0.0),
                    nfo_data.criticrating
                );
             }
        }

        Ok(target_path.to_string_lossy().to_string())
    }).await.map_err(|e| map_err(e))?;

    result.map(Json).map_err(map_err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleFavoritePayload {
    pub video_id: String,
}

pub async fn toggle_favorite(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ToggleFavoritePayload>,
) -> ApiResult<bool> {
    state.db.toggle_favorite(&payload.video_id).map(Json).map_err(map_err)
}

pub async fn get_all_genres(State(state): State<Arc<AppState>>) -> ApiResult<Vec<String>> {
    state.db.get_all_genres().map(Json).map_err(map_err)
}

pub async fn get_all_levels(State(state): State<Arc<AppState>>) -> ApiResult<Vec<String>> {
    state.db.get_all_levels().map(Json).map_err(map_err)
}

pub async fn get_stats(State(state): State<Arc<AppState>>) -> ApiResult<(usize, usize)> {
    let total = state.db.get_video_count().map_err(map_err)?;
    let favs = state.db.get_favorite_count().map_err(map_err)?;
    Ok(Json((total, favs)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchDbPayload {
    pub db_name: String,
}

pub async fn switch_database(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SwitchDbPayload>,
) -> ApiResult<()> {
    state.db.switch_database(&payload.db_name).map(Json).map_err(map_err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDbPayload {
    pub db_name: String,
}

pub async fn delete_database(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeleteDbPayload>,
) -> ApiResult<()> {
    let db_path = state.db.app_data_dir.join(format!("{}.db", payload.db_name));
    if db_path.exists() {
        std::fs::remove_file(&db_path).map_err(map_err)?;
    }
    let wal_path = state.db.app_data_dir.join(format!("{}.db-wal", payload.db_name));
    if wal_path.exists() {
        std::fs::remove_file(wal_path).ok();
    }
    let shm_path = state.db.app_data_dir.join(format!("{}.db-shm", payload.db_name));
    if shm_path.exists() {
        std::fs::remove_file(shm_path).ok();
    }
    Ok(Json(()))
}

#[derive(Deserialize)]
pub struct FileQuery {
    pub path: String,
}

#[derive(Deserialize)]
pub struct ListDirsPayload {
    pub path: Option<String>,
}

#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

pub async fn list_dirs(
    Json(payload): Json<ListDirsPayload>,
) -> ApiResult<Vec<DirEntry>> {
    let mut path_str = payload.path.unwrap_or_else(|| "/media".to_string());
    
    // Ensure path starts with /media
    if !path_str.starts_with("/media") {
        path_str = "/media".to_string();
    }
    
    println!("API: list_dirs request for path: {}", path_str);
    
    let path = Path::new(&path_str);
    
    if !path.exists() {
        return Err(map_err(format!("路徑不存在: {}", path_str)));
    }

    let mut entries_list = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            entries_list.push(DirEntry {
                name: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                path: p.to_string_lossy().to_string(),
                is_dir: p.is_dir(),
            });
        }
    }
    
    // Sort: directories first, then files, then alphabetically
    entries_list.sort_by(|a, b| {
        if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });
    
    println!("API: list_dirs found {} items", entries_list.len());
    Ok(Json(entries_list))
}

pub async fn serve_video_file(Query(query): Query<FileQuery>, req: Request) -> Result<axum::response::Response, ApiError> {
    let path = PathBuf::from(query.path);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }
    match ServeFile::new(path).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Error serving file".to_string())),
    }
}

#[derive(Deserialize)]
pub struct ImageQuery {
    pub path: String,
    pub id: Option<String>,
    pub thumb: Option<bool>,
}

pub async fn serve_image_file(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ImageQuery>,
    req: Request
) -> Result<axum::response::Response, ApiError> {
    let (parts, _) = req.into_parts();

    if query.thumb.unwrap_or(false) {
        if let Some(ref id) = query.id {
            let safe_id = id.replace("/", "_").replace("\\", "_").replace(":", "_");
            let thumb_path = state.db.app_data_dir.join("thumbnails").join(format!("{}.jpg", safe_id));
            if thumb_path.exists() {
                let req_for_thumb = Request::from_parts(parts.clone(), axum::body::Body::empty());
                match ServeFile::new(thumb_path).oneshot(req_for_thumb).await {
                    Ok(res) => return Ok(res.into_response()),
                    Err(_) => {} // Fallback
                }
            }
        }
    }

    let path = PathBuf::from(query.path);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }
    
    match ServeFile::new(path).oneshot(Request::from_parts(parts, axum::body::Body::empty())).await {
        Ok(res) => Ok(res.into_response()),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Error serving file".to_string())),
    }
}

pub async fn get_libraries(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Vec<Library>> {
    let path = state.db.app_data_dir.join("libraries.json");
    println!("API: get_libraries request. Path: {:?}", path);
    
    let mut libs: Vec<Library> = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                println!("API: read libraries.json success, length: {}", content.len());
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    println!("API: JSON Parse Error: {}", e);
                    Vec::new()
                })
            },
            Err(e) => {
                println!("API: Error reading libraries.json: {}", e);
                Vec::new()
            }
        }
    } else {
        println!("API: libraries.json not found at {:?}", path);
        Vec::new()
    };

    // 如果列表為空，建立一個預設的 StickPlay 媒體庫
    if libs.is_empty() {
        libs.push(Library {
            id: "default".to_string(),
            name: "StickPlay".to_string(),
            db_name: "stickplay".to_string(),
            paths: vec!["/media/stickplay".to_string()],
        });
        // 同步存回檔案
        let content = serde_json::to_string_pretty(&libs).map_err(map_err)?;
        std::fs::write(path, content).map_err(map_err)?;
    }

    Ok(Json(libs))
}


pub async fn save_libraries(
    State(state): State<Arc<AppState>>,
    Json(libs): Json<Vec<Library>>,
) -> ApiResult<()> {
    let path = state.db.app_data_dir.join("libraries.json");
    let content = serde_json::to_string_pretty(&libs).map_err(map_err)?;
    std::fs::write(path, content).map_err(map_err)?;
    Ok(Json(()))
}

