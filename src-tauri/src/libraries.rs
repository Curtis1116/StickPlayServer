use crate::{
    auth::Session,
    models::{Library, VideoFilter},
    security, AppState, ServerState,
};
use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Deserialize;
use std::{collections::HashSet, path::Path, sync::Arc};
use tower::ServiceExt;
type Error = (StatusCode, String);
fn bad(e: impl ToString) -> Error {
    (StatusCode::BAD_REQUEST, e.to_string())
}
pub fn read(state: &ServerState) -> Result<Vec<Library>, String> {
    serde_json::from_str(
        &std::fs::read_to_string(state.config.join("libraries.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn write(state: &ServerState, libs: &[Library]) -> Result<(), String> {
    let path = state.config.join("libraries.json");
    let tmp = state
        .config
        .join(format!("libraries-{}.tmp", crate::auth::token()));
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(libs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub fn initialize(state: &ServerState) -> Result<(), String> {
    if !state.config.join("libraries.json").exists() {
        write(
            state,
            &[Library {
                id: "default".into(),
                name: "StickPlay".into(),
                db_name: "stickplay".into(),
                paths: vec![],
            }],
        )?;
    }
    read(state)?;
    Ok(())
}
fn valid_db(name: &str) -> Result<(), String> {
    security::identifier(name)?;
    if name == "auth" {
        return Err("保留的資料庫名稱".into());
    }
    Ok(())
}
pub fn get(state: &ServerState, id: &str) -> Result<Arc<AppState>, String> {
    let lib = read(state)?
        .into_iter()
        .find(|l| l.id == id)
        .ok_or("找不到媒體庫，請重新整理")?;
    valid_db(&lib.db_name)?;
    let mut cache = state.libraries.lock().unwrap();
    if let Some(existing) = cache.get(id) {
        return Ok(existing.clone());
    }
    let db = crate::database::Database::open(state.config.clone(), &lib.db_name)
        .map_err(|e| e.to_string())?;
    let (event_tx, _) = tokio::sync::broadcast::channel(16);
    let entry = Arc::new(AppState {
        db,
        library_id: lib.id.clone(),
        watch_paths: std::sync::Mutex::new(lib.paths.into_iter().collect()),
        event_tx,
        operation: tokio::sync::Mutex::new(()),
    });
    cache.insert(id.into(), entry.clone());
    Ok(entry)
}
#[derive(Deserialize)]
struct Scope {
    #[serde(rename = "libraryId")]
    library_id: Option<String>,
}
pub async fn context(
    State(state): State<Arc<ServerState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Error> {
    let _gate = state.gate.read().await;
    let id = req
        .headers()
        .get("x-library-id")
        .and_then(|h| h.to_str().ok())
        .map(str::to_owned)
        .or_else(|| {
            Query::<Scope>::try_from_uri(req.uri())
                .ok()
                .and_then(|q| q.library_id.clone())
        })
        .ok_or_else(|| bad("請指定媒體庫"))?;
    let library = get(&state, &id).map_err(bad)?;
    req.extensions_mut().insert(library.clone());
    if req.method() == axum::http::Method::POST {
        let _operation = library.operation.lock().await;
        Ok(next.run(req).await)
    } else {
        Ok(next.run(req).await)
    }
}
pub async fn list(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<Library>>, Error> {
    let _guard = state.gate.read().await;
    read(&state).map(Json).map_err(bad)
}
pub async fn save(
    State(state): State<Arc<ServerState>>,
    Json(libs): Json<Vec<Library>>,
) -> Result<Json<()>, Error> {
    let _guard = state.gate.write().await;
    if libs.len() > 100 {
        return Err(bad("媒體庫數量過多"));
    }
    let old = read(&state).map_err(bad)?;
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for lib in &libs {
        security::identifier(&lib.id).map_err(bad)?;
        valid_db(&lib.db_name).map_err(bad)?;
        if !ids.insert(&lib.id) || !names.insert(&lib.db_name) || lib.name.trim().is_empty() {
            return Err(bad("媒體庫識別碼重複或名稱空白"));
        }
        if old
            .iter()
            .any(|l| l.id == lib.id && l.db_name != lib.db_name)
        {
            return Err(bad("不可變更已登記的資料庫名稱"));
        }
        for path in &lib.paths {
            security::media_path(Path::new(path)).map_err(bad)?;
        }
    }
    if old.iter().any(|l| !libs.iter().any(|n| n.id == l.id)) {
        return Err(bad("請使用刪除媒體庫功能移除媒體庫"));
    }
    write(&state, &libs).map_err(bad)?;
    for lib in libs {
        if let Some(entry) = state.libraries.lock().unwrap().get(&lib.id) {
            *entry.watch_paths.lock().unwrap() = lib.paths.into_iter().collect();
        }
    }
    Ok(Json(()))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbName {
    db_name: String,
}
pub async fn select(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<DbName>,
) -> Result<Json<String>, Error> {
    let _guard = state.gate.read().await;
    let lib = read(&state)
        .map_err(bad)?
        .into_iter()
        .find(|l| l.db_name == p.db_name)
        .ok_or_else(|| bad("找不到媒體庫"))?;
    get(&state, &lib.id).map_err(bad)?;
    Ok(Json(lib.id))
}
pub async fn delete(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<DbName>,
) -> Result<Json<Vec<Library>>, Error> {
    let _guard = state.gate.write().await;
    valid_db(&p.db_name).map_err(bad)?;
    let mut libs = read(&state).map_err(bad)?;
    let index = libs
        .iter()
        .position(|l| l.db_name == p.db_name)
        .ok_or_else(|| bad("找不到媒體庫"))?;
    let library = get(&state, &libs[index].id).map_err(bad)?;
    // A SQLite snapshot includes committed WAL data. Keep it for server-side restoration.
    let backup_dir = state.config.join("backups");
    std::fs::create_dir_all(&backup_dir).map_err(bad)?;
    let backup = backup_dir.join(format!("{}-{}.db", p.db_name, crate::auth::token()));
    library
        .db
        .conn
        .lock()
        .unwrap()
        .execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])
        .map_err(bad)?;
    let removed = libs.remove(index);
    write(&state, &libs).map_err(bad)?;
    state.libraries.lock().unwrap().remove(&removed.id);
    drop(library);
    // Retired files remain recoverable even if filesystem cleanup is interrupted.
    Ok(Json(libs))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketRequest {
    video_id: String,
}
pub async fn playback_ticket(
    State(state): State<Arc<ServerState>>,
    Extension(lib): Extension<Arc<AppState>>,
    Extension(session): Extension<Session>,
    Json(p): Json<TicketRequest>,
) -> Result<Json<String>, Error> {
    let videos = lib.db.query_videos(&VideoFilter::default()).map_err(bad)?;
    let video = videos
        .into_iter()
        .find(|v| v.id == p.video_id)
        .ok_or_else(|| bad("影片不存在"))?;
    let roots = lib
        .watch_paths
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let path = security::within(Path::new(&video.video_path), &roots).map_err(bad)?;
    security::extension(&path, security::VIDEOS).map_err(bad)?;
    state
        .auth
        .issue_ticket(&session.id, &lib.library_id, &path.to_string_lossy())
        .map(Json)
}
#[derive(Deserialize)]
pub struct TicketQuery {
    ticket: String,
}
pub async fn playback(
    State(state): State<Arc<ServerState>>,
    Query(q): Query<TicketQuery>,
    req: Request,
) -> Result<Response, Error> {
    let _guard = state.gate.read().await;
    let (id, path) = state.auth.playback(&q.ticket)?;
    let lib = get(&state, &id).map_err(bad)?;
    let roots = lib
        .watch_paths
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let path = security::within(Path::new(&path), &roots).map_err(bad)?;
    security::extension(&path, security::VIDEOS).map_err(bad)?;
    tower_http::services::ServeFile::new(path)
        .oneshot(req)
        .await
        .map(|r| r.into_response())
        .map_err(bad)
}

pub fn start_polling(state: Arc<ServerState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _gate = state.gate.read().await;
            for lib in read(&state).unwrap_or_default() {
                let Ok(entry) = get(&state, &lib.id) else {
                    continue;
                };
                let Ok(_operation) = entry.operation.try_lock() else {
                    continue;
                };
                let scan = entry.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    // Use the same recursive discovery as manual scans. Only remove a record
                    // after an explicit NotFound; an offline mount or I/O error preserves it.
                    for root in &lib.paths {
                        let Ok(root) = security::media_path(Path::new(root)) else {
                            continue;
                        };
                        if !root.is_dir() {
                            continue;
                        }
                        let known = scan
                            .db
                            .query_videos(&VideoFilter::default())
                            .unwrap_or_default();
                        for dir in walkdir::WalkDir::new(&root)
                            .into_iter()
                            .filter_map(Result::ok)
                            .filter(|e| e.file_type().is_dir())
                        {
                            if !known
                                .iter()
                                .any(|v| Path::new(&v.folder_path) == dir.path())
                            {
                                let _ =
                                    crate::scanner::scan_single_folder(&scan.db, dir.path(), false);
                            }
                        }
                    }
                    let _ = scan.db.prune_missing_videos(&lib.paths);
                    let _ = scan.event_tx.send("library_updated".into());
                })
                .await;
            }
        }
    });
}
