use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    Extension, Json,
};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::models::{VideoEntry, VideoFilter};
use crate::parser::{update_nfo, update_nfo_full};
use crate::scanner::scan_single_folder;
use crate::{security, AppState, ServerState};
fn checked(state: &AppState, path: &str) -> Result<PathBuf, ApiError> {
    let roots = state
        .watch_paths
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    security::within(Path::new(path), &roots).map_err(|e| (StatusCode::FORBIDDEN, e))
}
fn video(state: &AppState, id: &str) -> Result<VideoEntry, ApiError> {
    state
        .db
        .query_videos(&VideoFilter::default())
        .map_err(map_err)?
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "找不到影片".into()))
}
fn nfo_target(state: &AppState, v: &VideoEntry) -> Result<PathBuf, ApiError> {
    let path = v.nfo_path.clone().unwrap_or_else(|| {
        Path::new(&v.folder_path)
            .join("movie.nfo")
            .to_string_lossy()
            .into_owned()
    });
    let path = checked(state, &path)?;
    security::extension(&path, &["nfo"]).map_err(map_err)?;
    Ok(path)
}

type ApiError = (StatusCode, String);
type ApiResult<T> = Result<Json<T>, ApiError>;

fn map_err(e: impl ToString) -> ApiError {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// 離開作用域時自動清除 `is_scanning` 旗標，確保提前 return 或發生錯誤時旗標也不會卡住
struct ScanGuard<'a> {
    db: &'a crate::database::Database,
}

impl<'a> Drop for ScanGuard<'a> {
    fn drop(&mut self) {
        self.db.set_scanning(false);
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPathsPayload {
    pub paths: Vec<String>,
}

pub async fn scan_library(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<ScanPathsPayload>,
) -> ApiResult<usize> {
    for path in &payload.paths {
        checked(&state, path)?;
    }
    crate::scanner::scan_library_paths(&state.db, &payload.paths)
        .map(Json)
        .map_err(map_err)
}

pub async fn sync_watch_paths(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<ScanPathsPayload>,
) -> ApiResult<()> {
    let configured = state.watch_paths.lock().unwrap();
    if payload.paths.iter().any(|p| !configured.contains(p)) {
        return Err((StatusCode::BAD_REQUEST, "請先儲存媒體庫設定".into()));
    }
    Ok(Json(()))
}

/// SSE endpoint：讓前端即時收到媒體庫變更通知
pub async fn events(
    State(server): State<Arc<ServerState>>,
    Extension(session): Extension<crate::auth::Session>,
    Extension(state): Extension<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.event_tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            if !server.auth.active(&session.id) { break; }
            match tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv()).await {
                Err(_) => { yield Ok(Event::default().comment("keepalive")); continue; }
                Ok(message) => match message {
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
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<RescanPayload>,
) -> ApiResult<VideoEntry> {
    let resolved = checked(&state, &payload.folder_path)?;
    let dir = resolved.as_path();
    if dir.try_exists().map_err(map_err)? == false || !dir.is_dir() {
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

    let filter = VideoFilter::default();
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
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<QueryPayload>,
) -> ApiResult<Vec<VideoEntry>> {
    state
        .db
        .query_videos(&payload.filter)
        .map(Json)
        .map_err(map_err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFanartPayload {
    pub folder_path: String,
    pub video_path: String,
}

pub async fn get_fanart_path(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<GetFanartPayload>,
) -> ApiResult<String> {
    let resolved = checked(&state, &payload.folder_path)?;
    let folder = resolved.as_path();
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
            if path.is_file() && checked(&state, &path.to_string_lossy()).is_ok() {
                if let Some(ext) = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                {
                    if security::IMAGES.contains(&ext.as_str()) {
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
    #[serde(default)]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub year: Option<String>,
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
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<UpdateVideoInfoPayload>,
) -> ApiResult<String> {
    let original = video(&state, &payload.original_id)?;
    if payload.video_id.trim().is_empty()
        || payload.video_id.len() > 128
        || payload.actors.len() > 100
        || payload
            .genres
            .as_ref()
            .is_some_and(|genres| genres.len() > 100)
        || !(0..=100).contains(&payload.criticrating)
    {
        return Err((StatusCode::BAD_REQUEST, "影片資訊格式無效".into()));
    }
    let target = nfo_target(&state, &original)?;
    let mut guard = state.db.conn.lock().unwrap();
    let tx = guard.transaction().map_err(map_err)?;
    if original.id != payload.video_id {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM videos WHERE id=?1)",
                [&payload.video_id],
                |r| r.get(0),
            )
            .map_err(map_err)?;
        if exists {
            return Err((StatusCode::CONFLICT, "影片 ID 已存在，請使用其他 ID".into()));
        }
    }
    let level = payload.level.trim_end_matches(['X', 'x']);
    let year = payload.year.as_deref().unwrap_or(&original.year).trim();
    let source_genres = payload.genres.as_ref().unwrap_or(&original.genres);
    let mut genres = Vec::new();
    for genre in source_genres {
        let genre = genre.trim();
        if !genre.is_empty() && genre != "無碼" && !genres.iter().any(|item| item == genre) {
            genres.push(genre.to_string());
        }
    }
    tx.execute("UPDATE videos SET id=?1,title=?2,level=?3,rating=?4,criticrating=?5,year=?6,release_date=?7,date_added=?8,is_favorite=?9,nfo_path=?10,nfos_path=NULL WHERE id=?11",
        rusqlite::params![payload.video_id,payload.title,level,payload.criticrating as f64 / 10.0,payload.criticrating,year,payload.release_date,payload.date_added,payload.is_favorite,target.to_string_lossy(),original.id]).map_err(map_err)?;
    tx.execute("DELETE FROM video_actors WHERE video_id=?1", [&original.id])
        .map_err(map_err)?;
    tx.execute("DELETE FROM video_genres WHERE video_id=?1", [&original.id])
        .map_err(map_err)?;
    for actor in &payload.actors {
        tx.execute(
            "INSERT OR IGNORE INTO video_actors VALUES(?1,?2)",
            rusqlite::params![payload.video_id, actor],
        )
        .map_err(map_err)?;
    }
    for genre in &genres {
        tx.execute(
            "INSERT OR IGNORE INTO video_genres VALUES(?1,?2)",
            rusqlite::params![payload.video_id, genre],
        )
        .map_err(map_err)?;
    }
    if payload.is_uncensored {
        tx.execute(
            "INSERT OR IGNORE INTO video_genres VALUES(?1,'無碼')",
            [&payload.video_id],
        )
        .map_err(map_err)?;
    }
    update_nfo_full(
        &target,
        &payload.video_id,
        payload.rating,
        Some(payload.criticrating),
        &payload.actors,
        &payload.release_date,
        &payload.date_added,
        payload.is_uncensored,
        &payload.title,
        level,
        year,
        &genres,
    )
    .map_err(map_err)?;
    tx.commit().map_err(map_err)?;
    Ok(Json(target.to_string_lossy().into_owned()))
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
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<UpdateRatingPayload>,
) -> ApiResult<String> {
    let original = video(&state, &payload.video_id)?;
    let target = nfo_target(&state, &original)?;
    if !(0..=100).contains(&payload.criticrating) {
        return Err((StatusCode::BAD_REQUEST, "評分無效".into()));
    }
    update_nfo(
        &target,
        &original.id,
        payload.rating,
        Some(payload.criticrating),
        &original.date_added,
    )
    .map_err(map_err)?;
    state
        .db
        .update_rating(
            &original.id,
            payload.criticrating as f64 / 10.0,
            payload.criticrating,
        )
        .map_err(map_err)?;
    Ok(Json(target.to_string_lossy().into_owned()))
}

pub async fn get_folder_images(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<RescanPayload>, // reuse the folder_path payload
) -> ApiResult<Vec<String>> {
    let resolved = checked(&state, &payload.folder_path)?;
    let folder = resolved.as_path();
    if !folder.exists() || !folder.is_dir() {
        return Err(map_err("資料夾不存在"));
    }

    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && checked(&state, &path.to_string_lossy()).is_ok() {
                if let Some(ext) = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                {
                    if security::IMAGES.contains(&ext.as_str()) {
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
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<CropPayload>,
) -> ApiResult<String> {
    let original = video(
        &state,
        payload
            .video_id
            .as_deref()
            .ok_or_else(|| map_err("請指定影片"))?,
    )?;
    if checked(&state, &original.folder_path)? != checked(&state, &payload.output_folder)? {
        return Err((StatusCode::FORBIDDEN, "海報位置與影片不符".into()));
    }
    let image = checked(&state, &payload.image_path)?;
    security::extension(&image, security::IMAGES).map_err(map_err)?;
    checked(
        &state,
        &Path::new(&payload.output_folder)
            .join("poster.jpg")
            .to_string_lossy(),
    )?;
    let target_nfo = nfo_target(&state, &original)?;
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

        let nfo_path_opt = Some(target_nfo);
        let video_id_final = original.id;

        // 更新縮圖 (Thumbnail)
        if !video_id_final.is_empty() {
            let thumbnail_dir = state_cloned.db.thumbnail_dir();
            let _ = std::fs::create_dir_all(&thumbnail_dir);
            let safe_id = video_id_final
                .replace("/", "_")
                .replace("\\", "_")
                .replace(":", "_");
            let thumb_path = thumbnail_dir.join(format!("{}.jpg", safe_id));
            let _ = cropped.thumbnail(300, 450).save(&thumb_path);
        }

        if let Some(nfo_p) = nfo_path_opt {
            crate::parser::update_poster_nfo(&nfo_p)?;
        }
        state_cloned
            .db
            .conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE videos SET poster_path=?1 WHERE id=?2",
                rusqlite::params![target_path.to_string_lossy(), video_id_final],
            )
            .map_err(|e| e.to_string())?;

        Ok(target_path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| map_err(e))?;

    result.map(Json).map_err(map_err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleFavoritePayload {
    pub video_id: String,
}

pub async fn toggle_favorite(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<ToggleFavoritePayload>,
) -> ApiResult<bool> {
    state
        .db
        .toggle_favorite(&payload.video_id)
        .map(Json)
        .map_err(map_err)
}

pub async fn get_all_genres(Extension(state): Extension<Arc<AppState>>) -> ApiResult<Vec<String>> {
    state.db.get_all_genres().map(Json).map_err(map_err)
}

pub async fn get_all_levels(Extension(state): Extension<Arc<AppState>>) -> ApiResult<Vec<String>> {
    state.db.get_all_levels().map(Json).map_err(map_err)
}

pub async fn get_stats(Extension(state): Extension<Arc<AppState>>) -> ApiResult<(usize, usize)> {
    let total = state.db.get_video_count().map_err(map_err)?;
    let favs = state.db.get_favorite_count().map_err(map_err)?;
    Ok(Json((total, favs)))
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

pub async fn list_dirs(Json(payload): Json<ListDirsPayload>) -> ApiResult<Vec<DirEntry>> {
    let path_str = payload.path.unwrap_or_else(|| {
        std::env::var("STICKPLAY_MEDIA_DIR").unwrap_or_else(|_| "/media".into())
    });
    let resolved =
        security::media_path(Path::new(&path_str)).map_err(|e| (StatusCode::FORBIDDEN, e))?;
    let path = resolved.as_path();
    let mut entries_list = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if security::media_path(&p).is_err() {
                continue;
            }
            entries_list.push(DirEntry {
                name: p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
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

pub async fn serve_video_file(
    Extension(state): Extension<Arc<AppState>>,
    Query(query): Query<FileQuery>,
    req: Request,
) -> Result<axum::response::Response, ApiError> {
    let path = checked(&state, &query.path)?;
    if !state
        .db
        .query_videos(&VideoFilter::default())
        .map_err(map_err)?
        .iter()
        .any(|v| checked(&state, &v.video_path).ok().as_ref() == Some(&path))
    {
        return Err((StatusCode::NOT_FOUND, "影片尚未建立索引".into()));
    }
    security::extension(&path, security::VIDEOS).map_err(map_err)?;
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }
    match ServeFile::new(path).oneshot(req).await {
        Ok(res) => Ok(res.into_response()),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error serving file".to_string(),
        )),
    }
}

#[derive(Deserialize)]
pub struct ImageQuery {
    pub path: String,
    pub id: Option<String>,
    pub thumb: Option<bool>,
}

/// 強制瀏覽器每次都重新驗證（conditional GET）才能使用快取，而非直接沿用舊內容。
/// 海報／縮圖檔案會因裁切、重新索引等操作在背後被置換，若沒有這個標頭，瀏覽器可能
/// 完全不發出網路請求、直接沿用舊分頁快取下來的舊內容。ServeFile 本身已支援
/// Last-Modified／ETag 驗證，因此内容未變時仍會回應輕量的 304，不會犧牲頻寬。
fn with_no_cache(mut res: axum::response::Response) -> axum::response::Response {
    res.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    res
}

pub async fn serve_image_file(
    Extension(state): Extension<Arc<AppState>>,
    Query(query): Query<ImageQuery>,
    req: Request,
) -> Result<axum::response::Response, ApiError> {
    let path = checked(&state, &query.path)?;
    security::extension(&path, security::IMAGES).map_err(map_err)?;
    let (parts, _) = req.into_parts();

    if query.thumb.unwrap_or(false) {
        if let Some(ref id) = query.id {
            let safe_id = id.replace("/", "_").replace("\\", "_").replace(":", "_");

            if let Some(thumb_path) = state.db.resolve_thumbnail(&safe_id) {
                let req_for_thumb = Request::from_parts(parts.clone(), axum::body::Body::empty());
                match ServeFile::new(thumb_path).oneshot(req_for_thumb).await {
                    Ok(res) => return Ok(with_no_cache(res.into_response())),
                    Err(_) => {} // Fallback
                }
            }
        }
    }

    let path = checked(&state, &query.path)?;
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
    }

    match ServeFile::new(path)
        .oneshot(Request::from_parts(parts, axum::body::Body::empty()))
        .await
    {
        Ok(res) => Ok(with_no_cache(res.into_response())),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error serving file".to_string(),
        )),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveFolderPayload {
    pub video_id: String,
    pub current_folder_path: String,
    pub target_parent_folder: String,
}

pub async fn move_video_folder(
    Extension(state): Extension<Arc<AppState>>,
    Json(payload): Json<MoveFolderPayload>,
) -> ApiResult<VideoEntry> {
    let original = video(&state, &payload.video_id)?;
    let source = checked(&state, &original.folder_path)?;
    if source != checked(&state, &payload.current_folder_path)? {
        return Err((StatusCode::FORBIDDEN, "來源資料夾與影片不符".into()));
    }
    let target = checked(&state, &payload.target_parent_folder)?;
    if target.starts_with(&source) {
        return Err((StatusCode::BAD_REQUEST, "不能搬移到自身或子目錄".into()));
    }
    let current_dir = source.as_path();
    if !current_dir.exists() || !current_dir.is_dir() {
        return Err(map_err("來源資料夾不存在"));
    }

    let target_parent = target.as_path();
    if !target_parent.exists() || !target_parent.is_dir() {
        return Err(map_err("目的資料夾不存在"));
    }

    // 限制來源與目的都必須落在「目前使用中媒體庫」實際監控的路徑之內，而不是籠統地限制在 /media 之下。
    // 這樣一來：(1) 無法透過此 API 搬移到媒體庫監控範圍以外的任意主機目錄；
    // (2) 也無法搬移到「屬於其他媒體庫」的監控路徑，避免同一支影片被跨媒體庫重複索引。
    let watch_paths: Vec<String> = state.watch_paths.lock().unwrap().iter().cloned().collect();
    let within_watch_paths = |p: &str| watch_paths.iter().any(|wp| Path::new(p).starts_with(wp));
    if watch_paths.is_empty()
        || !within_watch_paths(&payload.current_folder_path)
        || !within_watch_paths(&payload.target_parent_folder)
    {
        return Err(map_err(
            "來源或目的路徑不在目前媒體庫的監控範圍內，無法搬移",
        ));
    }

    let folder_name = current_dir
        .file_name()
        .ok_or_else(|| map_err("無法解析來源資料夾名稱"))?;

    let new_dir = target_parent.join(folder_name);

    if new_dir.exists() {
        return Err(map_err("目的端已有同名資料夾"));
    }

    // 從搬移到重新索引完成為止設定掃描中旗標，避免 switch_database 在此期間插隊切換資料庫，
    // 導致索引寫入結果落到切換後的新資料庫而非原本預期的那一個（沿用 scan_library_paths 的保護慣例）。
    // 使用 RAII guard 確保無論哪個分支提前 return，旗標都會在函式結束時被清除。
    state.db.set_scanning(true);
    let _scan_guard = ScanGuard { db: &state.db };

    // 搬移資料夾
    std::fs::rename(current_dir, &new_dir).map_err(|e| map_err(format!("搬移失敗: {}", e)))?;

    // 重新掃描新位置。注意：此時故意不預先刪除舊路徑的資料庫紀錄——
    // 若影片 id 未變 (常見情況，因 id 是由資料夾名稱/NFO 內容決定而非父目錄路徑)，
    // upsert_video 會透過 ON CONFLICT(id) 就地更新 folder_path，並保留 is_favorite、
    // rating 等既有欄位；若在此之前先行刪除舊紀錄，就會退化成全新 INSERT 而遺失這些欄位。
    // 若重新索引失敗，舊紀錄會維持原狀（不會憑空消失），使用者仍可透過既有的
    // 「重新整理索引」功能自行修復。
    {
        let conn = state.db.conn.lock().unwrap();
        let remap = |p: &str| {
            current_dir
                .strip_prefix(current_dir)
                .ok()
                .and_then(|_| Path::new(p).strip_prefix(current_dir).ok())
                .map(|suffix| new_dir.join(suffix).to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string())
        };
        if let Err(e) = conn.execute(
            "UPDATE videos SET folder_path=?1,video_path=?2,nfo_path=?3,poster_path=?4 WHERE id=?5",
            rusqlite::params![
                new_dir.to_string_lossy(),
                remap(&original.video_path),
                original.nfo_path.as_deref().map(remap),
                original.poster_path.as_deref().map(remap),
                original.id
            ],
        ) {
            let _ = std::fs::rename(&new_dir, current_dir);
            return Err(map_err(e));
        }
    }
    if let Err(e) = scan_single_folder(&state.db, &new_dir, false) {
        return Err(map_err(format!(
            "MOVE_REINDEX_FAILED::資料夾已搬移完成，但重新索引失敗，請至新位置手動重新整理索引: {}",
            e
        )));
    }

    // 清除舊路徑殘留的孤兒紀錄：僅在重新掃描後 id 改變（因而未被就地更新）時才會命中，
    // 若 id 未變，上一步的就地更新已經把 folder_path 改成新路徑，這裡不會有任何符合的紀錄。
    {
        let conn = state.db.conn.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM videos WHERE folder_path = ?1",
            rusqlite::params![payload.current_folder_path],
        );
    }

    // 從資料庫撈出新的資料回傳，若撈不到代表該資料夾超出目前媒體庫的監控範圍
    let filter = VideoFilter {
        search: Some(payload.video_id),
        ..Default::default()
    };

    let videos = state.db.query_videos(&filter).map_err(map_err)?;
    let entry = videos
        .into_iter()
        .find(|v| v.folder_path == new_dir.to_string_lossy().to_string())
        .ok_or_else(|| map_err("MOVE_OUT_OF_RANGE::搬移成功，但該影片不在目前媒體庫的監控範圍內 (可能會從畫面上消失)"))?;

    Ok(Json(entry))
}
