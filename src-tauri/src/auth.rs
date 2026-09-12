use crate::ServerState;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
type Error = (StatusCode, String);
type Result<T> = std::result::Result<T, Error>;
const DAY: i64 = 86400;
fn err(code: StatusCode, msg: &str) -> Error {
    (code, msg.into())
}
fn internal(e: impl std::fmt::Display) -> Error {
    eprintln!("Auth storage: {e}");
    err(StatusCode::INTERNAL_SERVER_ERROR, "驗證服務暫時無法使用")
}
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
pub fn token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn password_hash(s: &str) -> Result<String> {
    Argon2::default()
        .hash_password(s.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|h| h.to_string())
        .map_err(internal)
}
fn verify(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).ok().is_some_and(|h| {
        Argon2::default()
            .verify_password(password.as_bytes(), &h)
            .is_ok()
    })
}
fn validate_password(s: &str) -> Result<()> {
    if !(12..=256).contains(&s.len()) {
        return Err(err(StatusCode::BAD_REQUEST, "密碼須為 12–256 位元組"));
    }
    Ok(())
}

pub struct AuthStore {
    conn: Mutex<Connection>,
    pub origin: String,
    secure: bool,
    limits: Mutex<HashMap<String, (i64, u32)>>,
    dummy_hash: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub csrf: String,
    pub username: String,
    pub expires_at: i64,
    pub remember: bool,
}
impl AuthStore {
    pub fn new(dir: &Path) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let origin = std::env::var("STICKPLAY_PUBLIC_ORIGIN")
            .unwrap_or_else(|_| "https://localhost".into())
            .trim_end_matches('/')
            .to_string();
        let parsed = url::Url::parse(&origin)?;
        let secure = parsed.scheme() == "https";
        if parsed.origin().ascii_serialization() != origin
            || (!secure
                && (parsed.scheme() != "http"
                    || std::env::var("STICKPLAY_ALLOW_INSECURE_HTTP").as_deref() != Ok("true")))
        {
            return Err("設定有效的 HTTPS STICKPLAY_PUBLIC_ORIGIN；HTTP 僅能明確設定 STICKPLAY_ALLOW_INSECURE_HTTP=true".into());
        }
        let db_path = dir.join("auth.db");
        if db_path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err("auth.db 不可為符號連結".into());
        }
        let conn = Connection::open(&db_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600))?;
        }
        conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS admin (id INTEGER PRIMARY KEY CHECK(id=1), username TEXT NOT NULL, password TEXT NOT NULL); CREATE TABLE IF NOT EXISTS setup (id INTEGER PRIMARY KEY CHECK(id=1), hash TEXT NOT NULL, expires INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, token_hash TEXT UNIQUE NOT NULL, csrf TEXT NOT NULL, created INTEGER NOT NULL, last_used INTEGER NOT NULL, expires INTEGER NOT NULL, remember INTEGER NOT NULL, device TEXT NOT NULL); CREATE TABLE IF NOT EXISTS tickets (hash TEXT PRIMARY KEY, session_id TEXT NOT NULL, library_id TEXT NOT NULL, path TEXT NOT NULL, expires INTEGER NOT NULL);")?;
        let exists: bool =
            conn.query_row("SELECT EXISTS(SELECT 1 FROM admin)", [], |r| r.get(0))?;
        if !exists {
            let code = token();
            conn.execute(
                "INSERT OR REPLACE INTO setup VALUES(1,?1,?2)",
                params![hash(&code), now() + 1800],
            )?;
            eprintln!("StickPlay 首次設定碼（30 分鐘內有效）：{code}");
        }
        Ok(Self {
            conn: Mutex::new(conn),
            origin,
            secure,
            limits: Mutex::new(HashMap::new()),
            dummy_hash: password_hash(&token()).map_err(|e| e.1)?,
        })
    }
    fn cookie_name(&self) -> &str {
        if self.secure {
            "__Host-stickplay_session"
        } else {
            "stickplay_session"
        }
    }
    fn cookie(&self, value: &str, age: Option<i64>) -> HeaderValue {
        HeaderValue::from_str(&format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax{}{}",
            self.cookie_name(),
            value,
            if self.secure { "; Secure" } else { "" },
            age.map(|a| format!("; Max-Age={a}")).unwrap_or_default()
        ))
        .unwrap()
    }
    fn raw_token<'a>(&self, headers: &'a HeaderMap) -> Option<&'a str> {
        headers
            .get(header::COOKIE)?
            .to_str()
            .ok()?
            .split(';')
            .find_map(|c| {
                c.trim()
                    .split_once('=')
                    .filter(|(k, _)| *k == self.cookie_name())
                    .map(|(_, v)| v)
            })
    }
    pub fn session(&self, headers: &HeaderMap) -> Result<Session> {
        let raw = self
            .raw_token(headers)
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "請先登入"))?;
        let conn = self.conn.lock().unwrap();
        let session = conn.query_row("SELECT s.id,s.csrf,a.username,s.expires,s.remember FROM sessions s CROSS JOIN admin a WHERE s.token_hash=?1 AND s.expires>?2 AND s.last_used + CASE WHEN s.remember=1 THEN 7776000 ELSE 43200 END > ?2",params![hash(raw),now()],|r|Ok(Session{id:r.get(0)?,csrf:r.get(1)?,username:r.get(2)?,expires_at:r.get(3)?,remember:r.get(4)?})).optional().map_err(internal)?.ok_or_else(||err(StatusCode::UNAUTHORIZED,"登入已失效，請重新登入"))?;
        conn.execute(
            "UPDATE sessions SET last_used=?1 WHERE id=?2 AND last_used < ?1-60",
            params![now(), session.id],
        )
        .map_err(internal)?;
        Ok(session)
    }
    pub fn active(&self, id: &str) -> bool {
        self.conn.lock().unwrap().query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1 AND expires>?2 AND last_used+CASE WHEN remember=1 THEN 7776000 ELSE 43200 END>?2)",params![id,now()],|r|r.get(0)).unwrap_or(false)
    }
    fn throttle(&self, key: String, max: u32) -> Result<()> {
        let mut limits = self.limits.lock().unwrap();
        let time = now();
        limits.retain(|_, (t, _)| *t + 300 > time);
        if limits.len() > 10000 {
            return Err(err(
                StatusCode::TOO_MANY_REQUESTS,
                "嘗試過於頻繁，請稍後再試",
            ));
        }
        let entry = limits.entry(key).or_insert((time, 0));
        entry.1 += 1;
        if entry.1 > max {
            return Err(err(
                StatusCode::TOO_MANY_REQUESTS,
                "嘗試過於頻繁，請 5 分鐘後再試",
            ));
        }
        Ok(())
    }
    fn issue(&self, conn: &Connection, remember: bool, device: &str) -> Result<Response> {
        let raw = token();
        let id = token();
        let expires = now() + if remember { 365 * DAY } else { 43200 };
        conn.execute("DELETE FROM sessions WHERE expires<=?1 OR last_used+CASE WHEN remember=1 THEN 7776000 ELSE 43200 END<=?1",[now()]).map_err(internal)?;
        conn.execute(
            "DELETE FROM tickets WHERE expires<=?1 OR session_id NOT IN (SELECT id FROM sessions)",
            [now()],
        )
        .map_err(internal)?;
        conn.execute(
            "INSERT INTO sessions VALUES(?1,?2,?3,?4,?4,?5,?6,?7)",
            params![
                id,
                hash(&raw),
                token(),
                now(),
                expires,
                remember,
                device.chars().take(160).collect::<String>()
            ],
        )
        .map_err(internal)?;
        let mut response = Json(serde_json::json!({"ok":true})).into_response();
        response.headers_mut().insert(
            header::SET_COOKIE,
            self.cookie(&raw, remember.then_some(90 * DAY)),
        );
        Ok(response)
    }
    pub fn playback(&self, ticket: &str) -> Result<(String, String)> {
        self.conn.lock().unwrap().query_row("SELECT t.library_id,t.path FROM tickets t JOIN sessions s ON s.id=t.session_id WHERE t.hash=?1 AND t.expires>?2 AND s.expires>?2 AND s.last_used+CASE WHEN s.remember=1 THEN 7776000 ELSE 43200 END>?2",params![hash(ticket),now()],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(internal)?.ok_or_else(||err(StatusCode::UNAUTHORIZED,"播放連結已失效，請重新播放"))
    }
    pub fn issue_ticket(&self, session: &str, library: &str, path: &str) -> Result<String> {
        let raw = token();
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tickets WHERE expires<=?1", [now()])
            .map_err(internal)?;
        conn.execute(
            "INSERT INTO tickets VALUES(?1,?2,?3,?4,?5)",
            params![hash(&raw), session, library, path, now() + 6 * 3600],
        )
        .map_err(internal)?;
        Ok(format!("/api/playback?ticket={raw}"))
    }
}

pub async fn guard(
    State(state): State<Arc<ServerState>>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let path = req
        .extensions()
        .get::<axum::extract::OriginalUri>()
        .map(|u| u.0.path())
        .unwrap_or(req.uri().path())
        .to_string();
    let public = matches!(
        path.as_str(),
        "/api/auth/status"
            | "/api/auth/setup"
            | "/api/auth/login"
            | "/api/auth/recover"
            | "/api/playback"
    );
    let mut renewal = None;
    if !public {
        let session = state.auth.session(req.headers())?;
        if req.method() != axum::http::Method::GET
            && req.method() != axum::http::Method::HEAD
            && req
                .headers()
                .get("x-csrf-token")
                .and_then(|h| h.to_str().ok())
                != Some(&session.csrf)
        {
            return Err(err(StatusCode::FORBIDDEN, "驗證資訊已更新，請重新整理"));
        }
        if session.remember {
            renewal = state.auth.raw_token(req.headers()).map(|v| {
                state
                    .auth
                    .cookie(v, Some((session.expires_at - now()).min(90 * DAY)))
            });
        }
        req.extensions_mut().insert(session);
    }
    if req.method() != axum::http::Method::GET && req.method() != axum::http::Method::HEAD {
        if req
            .headers()
            .get(header::ORIGIN)
            .and_then(|h| h.to_str().ok())
            != Some(&state.auth.origin)
        {
            return Err(err(StatusCode::FORBIDDEN, "請從設定的網站網址操作"));
        }
        if !req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .is_some_and(|h| h.split(';').next() == Some("application/json"))
        {
            return Err(err(StatusCode::UNSUPPORTED_MEDIA_TYPE, "必須使用 JSON"));
        }
    }
    let mut response = next.run(req).await;
    if let Some(cookie) = renewal {
        if !response.headers().contains_key(header::SET_COOKIE) {
            response.headers_mut().insert(header::SET_COOKIE, cookie);
        }
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

pub async fn status(State(state): State<Arc<ServerState>>) -> Result<Json<serde_json::Value>> {
    let conn = state.auth.conn.lock().unwrap();
    let setup: bool = conn
        .query_row("SELECT NOT EXISTS(SELECT 1 FROM admin)", [], |r| r.get(0))
        .map_err(internal)?;
    Ok(Json(serde_json::json!({"setupRequired":setup})))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    username: String,
    password: String,
    code: Option<String>,
    #[serde(default)]
    remember: bool,
}
async fn credentials(
    state: Arc<ServerState>,
    headers: HeaderMap,
    peer: SocketAddr,
    p: Credentials,
    mode: &str,
) -> Result<Response> {
    state.auth.throttle(format!("ip:{}", peer.ip()), 10)?;
    state.auth.throttle("account".into(), 50)?;
    let mode = mode.to_string();
    tokio::task::spawn_blocking(move || {
        if p.password.len() > 256 || p.username.len() > 64 {
            return Err(err(StatusCode::BAD_REQUEST, "帳號或密碼格式無效"));
        }
        let mut conn = state.auth.conn.lock().unwrap();
        let tx = conn.transaction().map_err(internal)?;
        let admin: Option<(String, String)> = tx
            .query_row("SELECT username,password FROM admin", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()
            .map_err(internal)?;
        if mode == "login" {
            let encoded = admin
                .as_ref()
                .map(|a| a.1.as_str())
                .unwrap_or(&state.auth.dummy_hash);
            let valid = verify(&p.password, encoded);
            if !valid || !admin.as_ref().is_some_and(|a| a.0 == p.username) {
                return Err(err(StatusCode::UNAUTHORIZED, "帳號或密碼不正確"));
            }
        } else {
            validate_password(&p.password)?;
            if p.username.trim().is_empty() {
                return Err(err(StatusCode::BAD_REQUEST, "請輸入帳號"));
            }
            if mode == "setup" && admin.is_some() {
                return Err(err(StatusCode::CONFLICT, "管理員已建立"));
            }
            let valid: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM setup WHERE hash=?1 AND expires>?2)",
                    params![hash(p.code.as_deref().unwrap_or("")), now()],
                    |r| r.get(0),
                )
                .map_err(internal)?;
            if !valid {
                return Err(err(StatusCode::UNAUTHORIZED, "設定碼錯誤或已過期"));
            }
            tx.execute(
                "INSERT OR REPLACE INTO admin VALUES(1,?1,?2)",
                params![p.username.trim(), password_hash(&p.password)?],
            )
            .map_err(internal)?;
            tx.execute_batch("DELETE FROM setup; DELETE FROM sessions; DELETE FROM tickets;")
                .map_err(internal)?;
        }
        let response = state.auth.issue(
            &tx,
            p.remember,
            headers
                .get(header::USER_AGENT)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("瀏覽器"),
        )?;
        tx.commit().map_err(internal)?;
        Ok(response)
    })
    .await
    .map_err(internal)?
}
pub async fn login(
    State(s): State<Arc<ServerState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(p): Json<Credentials>,
) -> Result<Response> {
    credentials(s, headers, peer, p, "login").await
}
pub async fn setup(
    State(s): State<Arc<ServerState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(p): Json<Credentials>,
) -> Result<Response> {
    credentials(s, headers, peer, p, "setup").await
}
pub async fn recover(
    State(s): State<Arc<ServerState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(p): Json<Credentials>,
) -> Result<Response> {
    credentials(s, headers, peer, p, "recover").await
}
pub async fn me(Extension(s): Extension<Session>) -> Json<Session> {
    Json(s)
}
pub async fn logout(
    State(state): State<Arc<ServerState>>,
    Extension(s): Extension<Session>,
) -> Result<Response> {
    state
        .auth
        .conn
        .lock()
        .unwrap()
        .execute("DELETE FROM sessions WHERE id=?1", [s.id])
        .map_err(internal)?;
    let mut response = Json(()).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, state.auth.cookie("", Some(0)));
    Ok(response)
}
pub async fn devices(
    State(state): State<Arc<ServerState>>,
    Extension(s): Extension<Session>,
) -> Result<Json<serde_json::Value>> {
    let conn = state.auth.conn.lock().unwrap();
    let mut stmt=conn.prepare("SELECT id,device,created,last_used FROM sessions WHERE expires>?1 AND last_used+CASE WHEN remember=1 THEN 7776000 ELSE 43200 END>?1 ORDER BY last_used DESC").map_err(internal)?;
    let rows=stmt.query_map([now()],|r|{let id:String=r.get(0)?;Ok(serde_json::json!({"current":id==s.id,"id":id,"device":r.get::<_,String>(1)?,"created":r.get::<_,i64>(2)?,"lastUsed":r.get::<_,i64>(3)?}))}).map_err(internal)?.collect::<std::result::Result<Vec<_>,_>>().map_err(internal)?;
    Ok(Json(serde_json::json!(rows)))
}
#[derive(Deserialize)]
pub struct Revoke {
    id: Option<String>,
}
pub async fn revoke(
    State(state): State<Arc<ServerState>>,
    Extension(s): Extension<Session>,
    Json(p): Json<Revoke>,
) -> Result<Json<()>> {
    let conn = state.auth.conn.lock().unwrap();
    if let Some(id) = p.id {
        conn.execute("DELETE FROM sessions WHERE id=?1", [id])
            .map_err(internal)?;
    } else {
        conn.execute("DELETE FROM sessions WHERE id!=?1", [s.id])
            .map_err(internal)?;
    }
    Ok(Json(()))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordChange {
    current_password: String,
    new_password: String,
}
pub async fn change_password(
    State(state): State<Arc<ServerState>>,
    Extension(s): Extension<Session>,
    Json(p): Json<PasswordChange>,
) -> Result<Response> {
    state.auth.throttle(format!("password:{}", s.id), 5)?;
    validate_password(&p.new_password)?;
    if p.current_password.len() > 256 {
        return Err(err(StatusCode::BAD_REQUEST, "密碼格式無效"));
    }
    tokio::task::spawn_blocking(move || {
        let mut conn = state.auth.conn.lock().unwrap();
        let tx = conn.transaction().map_err(internal)?;
        let old: String = tx
            .query_row("SELECT password FROM admin", [], |r| r.get(0))
            .map_err(internal)?;
        if !verify(&p.current_password, &old) {
            return Err(err(StatusCode::UNAUTHORIZED, "目前密碼不正確"));
        }
        tx.execute(
            "UPDATE admin SET password=?1",
            [password_hash(&p.new_password)?],
        )
        .map_err(internal)?;
        tx.execute_batch("DELETE FROM sessions; DELETE FROM tickets;")
            .map_err(internal)?;
        let res = state.auth.issue(&tx, s.remember, "變更密碼的裝置")?;
        tx.commit().map_err(internal)?;
        Ok(res)
    })
    .await
    .map_err(internal)?
}
// Run inside the container; no public endpoint can generate a recovery code.
pub fn recovery_code(dir: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open_with_flags(
        dir.join("auth.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    )?;
    let code = token();
    conn.execute(
        "INSERT OR REPLACE INTO setup VALUES(1,?1,?2)",
        params![hash(&code), now() + 1800],
    )?;
    println!("一次性復原／設定碼（30 分鐘有效）：{code}");
    Ok(())
}
