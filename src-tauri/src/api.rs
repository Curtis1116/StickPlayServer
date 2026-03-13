use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tower_http::services::ServeFile;
use tower::ServiceExt;

use crate::database::Database;
use crate::models::{VideoEntry, VideoFilter, Library};
use crate::parser::{update_nfos, update_nfos_full};
use crate::scanner::{scan_library_paths, scan_single_folder};
use crate::AppState;
use notify::{Watcher, RecursiveMode};

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
    println!("API: sync_watch_paths: {:?}", payload.paths);
    
    let mut watch_paths = state.watch_paths.lock().unwrap();
    let mut watcher = state.watcher.lock().unwrap();
    
    if let Some(ref mut w) = *watcher {
        // Unwatch old paths
        for p in watch_paths.iter() {
            let _ = w.unwatch(Path::new(p));
        }
        
        // Clear and add new paths
        watch_paths.clear();
        for p in payload.paths {
            if !p.is_empty() {
                if let Ok(_) = w.watch(Path::new(&p), RecursiveMode::Recursive) {
                    watch_paths.insert(p.clone());
                }
            }
        }
    }
    
    Ok(Json(()))
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

    if let Err(e) = scan_single_folder(&state.db, dir, true) {
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
    pub actors: Vec<String>,
    pub release_date: String,
    pub date_added: String,
    pub is_favorite: bool,
    pub is_uncensored: bool,
    pub video_path: String,
    pub folder_path: String,
    pub poster_path: Option<String>,
    pub nfo_path: Option<String>,
    pub nfos_path: Option<String>,
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

    let target_nfos = if let Some(ref existing) = payload.nfos_path {
        existing.clone()
    } else if let Some(ref nfo) = payload.nfo_path {
        let nfo_p = Path::new(nfo);
        nfo_p.with_extension("nfos").to_string_lossy().to_string()
    } else {
        let folder_p = Path::new(&payload.folder_path);
        folder_p
            .join(format!("{}.nfos", payload.video_id))
            .to_string_lossy()
            .to_string()
    };

    let nfos_p = Path::new(&target_nfos);
    update_nfos_full(
        nfos_p,
        &payload.video_id,
        payload.rating,
        &payload.level,
        &payload.actors,
        &payload.release_date,
        &payload.date_added,
        payload.is_uncensored,
        payload.nfo_path.as_deref(),
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
        payload.nfo_path.as_deref(),
        Some(&target_nfos),
        &payload.actors,
        &new_genres,
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

    Ok(Json(target_nfos))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRatingPayload {
    pub video_id: String,
    pub rating: f64,
    pub nfo_path: Option<String>,
    pub nfos_path: Option<String>,
    pub folder_path: Option<String>,
}

pub async fn update_rating(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateRatingPayload>,
) -> ApiResult<String> {
    state.db.update_rating(&payload.video_id, payload.rating)
        .map_err(map_err)?;

    let target_nfos = if let Some(ref existing) = payload.nfos_path {
        existing.clone()
    } else if let Some(ref nfo) = payload.nfo_path {
        let nfo_p = Path::new(nfo);
        nfo_p.with_extension("nfos").to_string_lossy().to_string()
    } else if let Some(ref folder) = payload.folder_path {
        let folder_p = Path::new(folder);
        folder_p
            .join(format!("{}.nfos", payload.video_id))
            .to_string_lossy()
            .to_string()
    } else {
        return Ok(Json(String::new()));
    };

    let nfos_p = Path::new(&target_nfos);
    update_nfos(nfos_p, &payload.video_id, payload.rating, payload.nfo_path.as_deref()).map_err(map_err)?;

    {
        let conn = state.db.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET nfos_path = ?1 WHERE id = ?2",
            rusqlite::params![target_nfos, payload.video_id],
        )
        .map_err(map_err)?;
    }

    Ok(Json(target_nfos))
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

pub async fn serve_image_file(Query(query): Query<FileQuery>, req: Request) -> Result<axum::response::Response, ApiError> {
    let path = PathBuf::from(query.path);
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }
    match ServeFile::new(path).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, "Error serving file".to_string())),
    }
}

pub async fn get_libraries(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Vec<Library>> {
    let path = state.db.app_data_dir.join("libraries.json");
    if !path.exists() {
        return Ok(Json(Vec::new()));
    }
    let content = std::fs::read_to_string(path).map_err(map_err)?;
    let libs = serde_json::from_str(&content).map_err(map_err)?;
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
