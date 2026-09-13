use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use crate::models::{VideoEntry, VideoFilter};

/// `conn` 與目前所屬的 `name` 綁在一起、由同一把鎖保護，確保「切換資料庫」時
/// 兩者一定同時可見同一個狀態，不會有其他執行緒讀到「conn 已切換但 name 還沒切換」
/// （或反過來）的不一致中間狀態。對外仍可直接呼叫 Connection 的方法（透過 Deref）。
pub struct DbConn {
    conn: Connection,
    name: String,
}

impl std::ops::Deref for DbConn {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.conn
    }
}

impl std::ops::DerefMut for DbConn {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

/// 資料庫封裝 — 使用 Mutex 確保線程安全
pub struct Database {
    pub conn: Mutex<DbConn>,
    pub app_data_dir: PathBuf,
    /// 掃描中旗標：防止掃描進行中切換資料庫導致索引寫入錯誤資料庫
    is_scanning: AtomicBool,
    /// 快取「目前資料庫的縮圖目錄是否已建立」，避免 serve_image_file 這種高頻率的
    /// 熱路徑每次請求都重複執行一次阻塞式的 create_dir_all 系統呼叫；切換資料庫時會重置。
    thumb_dir_ready: AtomicBool,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> SqlResult<Self> {
        Self::open(app_data_dir, "stickplay")
    }

    pub fn open(app_data_dir: PathBuf, name: &str) -> SqlResult<Self> {
        crate::security::identifier(name).map_err(|_| rusqlite::Error::InvalidQuery)?;
        if name == "auth" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        std::fs::create_dir_all(&app_data_dir).ok();
        let conn = Self::create_connection(&app_data_dir, name)?;
        Ok(Self {
            conn: Mutex::new(DbConn {
                conn,
                name: name.into(),
            }),
            app_data_dir,
            is_scanning: AtomicBool::new(false),
            thumb_dir_ready: AtomicBool::new(false),
        })
    }

    /// 建立與特定資料庫的連線，並確保資料表存在
    fn create_connection(app_data_dir: &PathBuf, db_name: &str) -> SqlResult<Connection> {
        let db_path = app_data_dir.join(format!("{}.db", db_name));
        if db_path
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let conn = Connection::open(db_path)?;

        // 啟用 WAL 模式以提升並行效能
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS videos (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT '',
                level TEXT NOT NULL DEFAULT '',
                rating REAL NOT NULL DEFAULT 0.0,
                year TEXT NOT NULL DEFAULT '',
                release_date TEXT NOT NULL DEFAULT '',
                date_added TEXT NOT NULL DEFAULT '',
                video_path TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                poster_path TEXT,
                nfo_path TEXT,
                nfos_path TEXT,
                is_favorite INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS video_actors (
                video_id TEXT NOT NULL,
                actor_name TEXT NOT NULL,
                PRIMARY KEY (video_id, actor_name),
                FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS video_genres (
                video_id TEXT NOT NULL,
                genre TEXT NOT NULL,
                PRIMARY KEY (video_id, genre),
                FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
            );
            ",
        )?;

        // Migration: 若舊版 DB 缺少 nfos_path 欄位則補上 (保持相容性但不再主動使用)
        conn.execute_batch("ALTER TABLE videos ADD COLUMN nfos_path TEXT;")
            .ok();

        // Migration: 新增 criticrating 欄位
        conn.execute_batch(
            "ALTER TABLE videos ADD COLUMN criticrating INTEGER NOT NULL DEFAULT 0;",
        )
        .ok();

        // Migration: 新增 NFO year 欄位
        conn.execute_batch("ALTER TABLE videos ADD COLUMN year TEXT NOT NULL DEFAULT ''; ")
            .ok();

        Ok(conn)
    }

    /// 取得目前資料庫專屬的縮圖路徑
    pub fn thumbnail_dir(&self) -> PathBuf {
        let name = self.conn.lock().unwrap().name.clone();
        let dir_val = if name == "stickplay" {
            // 保留預設資料庫的縮圖路徑相容性
            self.app_data_dir.join("thumbnails")
        } else {
            self.app_data_dir.join(format!("thumbnails_{}", name))
        };
        // 只在切換資料庫後的第一次呼叫才真正執行 create_dir_all，避免每一次縮圖請求
        // 都重複做一次阻塞式的檔案系統呼叫
        if !self.thumb_dir_ready.swap(true, Ordering::SeqCst) {
            std::fs::create_dir_all(&dir_val).ok();
        }
        dir_val
    }

    /// 找出某個影片 id 實際可用的縮圖路徑：優先使用「目前資料庫專屬」的縮圖目錄，
    /// 若找不到則回退查詢舊版共用的 `thumbnails/` 資料夾。
    ///
    /// 背景：在引入「每個媒體庫獨立縮圖目錄」之前，所有媒體庫共用同一個 thumbnails/
    /// 資料夾。既有安裝升級後，之前已建立索引、尚未被重新掃描的影片，其縮圖仍留在
    /// 舊的共用資料夾內；若不回退查詢，會導致這些縮圖全部「憑空消失」，改為顯示
    /// 未經壓縮的原始海報圖。
    pub fn resolve_thumbnail(&self, safe_id: &str) -> Option<PathBuf> {
        let filename = format!("{}.jpg", safe_id);

        let current = self.thumbnail_dir().join(&filename);
        if current.is_file()
            && current.canonicalize().ok().is_some_and(|p| {
                self.thumbnail_dir()
                    .canonicalize()
                    .ok()
                    .is_some_and(|root| p.starts_with(root))
            })
        {
            return Some(current);
        }

        let legacy = self.app_data_dir.join("thumbnails").join(&filename);
        if legacy.is_file()
            && legacy.canonicalize().ok().is_some_and(|p| {
                self.app_data_dir
                    .join("thumbnails")
                    .canonicalize()
                    .ok()
                    .is_some_and(|root| p.starts_with(root))
            })
        {
            return Some(legacy);
        }

        None
    }

    /// 設定掃描旗標（由 scanner 呼叫）
    pub fn set_scanning(&self, scanning: bool) {
        self.is_scanning.store(scanning, Ordering::SeqCst);
    }

    /// 回傳目前是否正在掃描
    pub fn is_scanning(&self) -> bool {
        self.is_scanning.load(Ordering::SeqCst)
    }

    /// 插入或更新影片記錄
    pub fn upsert_video(
        &self,
        id: &str,
        title: &str,
        level: &str,
        rating: Option<f64>,
        year: &str,
        release_date: &str,
        date_added: &str,
        video_path: &str,
        folder_path: &str,
        poster_path: Option<&str>,
        nfo_path: Option<&str>,
        nfos_path: Option<&str>,
        actors: &[String],
        genres: &[String],
        criticrating: i32,
    ) -> SqlResult<()> {
        let mut guard = self.conn.lock().unwrap();
        let conn = guard.transaction()?;
        let collision: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM videos WHERE id=?1 AND folder_path!=?2)",
            params![id, folder_path],
            |r| r.get(0),
        )?;
        if collision {
            return Err(rusqlite::Error::InvalidParameterName(
                "影片 ID 已存在".into(),
            ));
        }
        // 清除相同資料夾但 ID 不同的舊記錄 (避免修改 ID 擷取邏輯後產生重複記錄)
        conn.execute(
            "DELETE FROM videos WHERE folder_path = ?1 AND id != ?2",
            params![folder_path, id],
        )?;

        conn.execute(
            "INSERT INTO videos (id, title, level, rating, year, release_date, date_added, video_path, folder_path, poster_path, nfo_path, nfos_path, criticrating)
             VALUES (?1, ?2, ?3, COALESCE(?4, 0.0), ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                level = excluded.level,
                rating = COALESCE(excluded.rating, videos.rating),
                year = excluded.year,
                release_date = excluded.release_date,
                date_added = excluded.date_added,
                video_path = excluded.video_path,
                folder_path = excluded.folder_path,
                poster_path = excluded.poster_path,
                nfo_path = excluded.nfo_path,
                nfos_path = excluded.nfos_path,
                criticrating = excluded.criticrating",
            params![id, title, level, rating, year, release_date, date_added, video_path, folder_path, poster_path, nfo_path, nfos_path, criticrating],
        )?;

        // 清除舊的 actors / genres 然後重新插入
        conn.execute("DELETE FROM video_actors WHERE video_id = ?1", params![id])?;
        for actor in actors {
            conn.execute(
                "INSERT OR IGNORE INTO video_actors (video_id, actor_name) VALUES (?1, ?2)",
                params![id, actor],
            )?;
        }

        conn.execute("DELETE FROM video_genres WHERE video_id = ?1", params![id])?;
        for genre in genres {
            conn.execute(
                "INSERT OR IGNORE INTO video_genres (video_id, genre) VALUES (?1, ?2)",
                params![id, genre],
            )?;
        }

        conn.commit()?;
        Ok(())
    }

    /// 依據篩選條件查詢影片（核心查詢 — 排序/篩選/搜尋均在 SQL 層處理）
    pub fn query_videos(&self, filter: &VideoFilter) -> SqlResult<Vec<VideoEntry>> {
        let conn = self.conn.lock().unwrap();

        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        // 等級篩選
        if let Some(ref levels) = filter.levels {
            if !levels.is_empty() {
                let mut level_conditions = Vec::new();
                for level in levels {
                    if level == "無分級" {
                        level_conditions.push("v.level = ''".to_string());
                    } else {
                        level_conditions.push(format!("v.level = ?{}", param_values.len() + 1));
                        param_values.push(Box::new(level.clone()));
                    }
                }
                conditions.push(format!("({})", level_conditions.join(" OR ")));
            }
        }

        // 類型篩選
        if let Some(ref genres) = filter.genres {
            if !genres.is_empty() {
                let mut genre_conditions = Vec::new();
                for genre in genres {
                    genre_conditions.push(format!(
                        "EXISTS (SELECT 1 FROM video_genres vg WHERE vg.video_id = v.id AND vg.genre = ?{})",
                        param_values.len() + 1
                    ));
                    param_values.push(Box::new(genre.clone()));
                }
                conditions.push(format!("({})", genre_conditions.join(" OR ")));
            }
        }

        // 我的最愛篩選
        if let Some(true) = filter.favorites_only {
            conditions.push("v.is_favorite = 1".to_string());
        }

        // 搜尋（模糊匹配 title, id, actors）
        if let Some(ref search) = filter.search {
            if !search.is_empty() {
                let idx = param_values.len() + 1;
                conditions.push(format!(
                    "(v.title LIKE ?{idx} OR v.id LIKE ?{idx} OR EXISTS (SELECT 1 FROM video_actors va WHERE va.video_id = v.id AND va.actor_name LIKE ?{idx}))"
                ));
                param_values.push(Box::new(format!("%{}%", search)));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // 排序
        let order_clause = {
            let sort_by = filter.sort_by.as_deref().unwrap_or("date_added");
            let sort_order = filter.sort_order.as_deref().unwrap_or("DESC");
            let order = if sort_order.eq_ignore_ascii_case("ASC") {
                "ASC"
            } else {
                "DESC"
            };
            let column = match sort_by {
                "title" => "v.title",
                "rating" => "v.criticrating",
                "release_date" => "v.release_date",
                "date_added" => "v.date_added",
                "id" => "v.id",
                "level" => "v.level",
                "actor" => "(SELECT actor_name FROM video_actors WHERE video_id = v.id ORDER BY rowid LIMIT 1)",
                _ => "v.date_added",
            };
            format!("ORDER BY {} {}", column, order)
        };

        let query = format!(
            "SELECT v.id, v.title, v.level, v.rating, v.year, v.release_date, v.date_added,
                    v.video_path, v.folder_path, v.poster_path, v.nfo_path, v.nfos_path, v.is_favorite, v.criticrating
             FROM videos v
             {} {}",
            where_clause, order_clause
        );

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&query)?;
        let video_rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, bool>(12)?,
                row.get::<_, i32>(13)?,
            ))
        })?;

        let mut videos = Vec::new();
        for row in video_rows {
            let (
                id,
                title,
                level,
                rating,
                year,
                release_date,
                date_added,
                video_path,
                folder_path,
                poster_path,
                nfo_path,
                nfos_path,
                is_favorite,
                criticrating,
            ) = row?;

            // 查詢關聯的 actors
            let actors: Vec<String> = {
                let mut stmt =
                    conn.prepare("SELECT actor_name FROM video_actors WHERE video_id = ?1")?;
                let rows = stmt.query_map(params![&id], |row| row.get(0))?;
                let result: Vec<String> = rows.filter_map(|r| r.ok()).collect();
                result
            };

            // 查詢關聯的 genres
            let genres: Vec<String> = {
                let mut stmt =
                    conn.prepare("SELECT genre FROM video_genres WHERE video_id = ?1")?;
                let rows = stmt.query_map(params![&id], |row| row.get(0))?;
                let result: Vec<String> = rows.filter_map(|r| r.ok()).collect();
                result
            };

            videos.push(VideoEntry {
                id,
                title,
                actors,
                genres,
                level,
                rating,
                release_date,
                date_added,
                video_path,
                folder_path,
                poster_path,
                nfo_path,
                nfos_path,
                is_favorite,
                criticrating,
                year,
            });
        }

        Ok(videos)
    }

    /// 更新影片評分與評論評分
    pub fn update_rating(&self, video_id: &str, rating: f64, criticrating: i32) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET rating = ?1, criticrating = ?2 WHERE id = ?3",
            params![rating, criticrating, video_id],
        )?;
        Ok(())
    }

    /// 切換我的最愛狀態，回傳新狀態
    pub fn toggle_favorite(&self, video_id: &str) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE videos SET is_favorite = CASE WHEN is_favorite = 1 THEN 0 ELSE 1 END WHERE id = ?1",
            params![video_id],
        )?;
        let new_state: bool = conn.query_row(
            "SELECT is_favorite FROM videos WHERE id = ?1",
            params![video_id],
            |row| row.get(0),
        )?;
        Ok(new_state)
    }

    /// 取得所有不重複的 genres
    pub fn get_all_genres(&self) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT DISTINCT genre FROM video_genres ORDER BY genre")?;
        let genres = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(genres)
    }

    /// 取得所有不重複的 levels
    pub fn get_all_levels(&self) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT DISTINCT level FROM videos WHERE level != '' ORDER BY level")?;
        let levels = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(levels)
    }

    /// 取得影片總數
    pub fn get_video_count(&self) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM videos", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// 取得我的最愛數量
    pub fn get_favorite_count(&self) -> SqlResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM videos WHERE is_favorite = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// 清除對應資料夾已不存在於檔案系統，或是不在目前媒體庫範圍內的影片記錄
    pub fn prune_missing_videos(&self, valid_library_paths: &[String]) -> SqlResult<usize> {
        // 先取得所有影片的 id, folder_path 與 video_path
        let videos: Vec<(String, String, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT id, folder_path, video_path FROM videos")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // 檢查影片檔實體是否不存在，或是其所在主目錄已經不在 valid_library_paths 內
        let mut missing_ids = Vec::new();
        for (id, folder_path, video_path) in videos {
            let f_path = std::path::Path::new(&folder_path);
            let v_path = std::path::Path::new(&video_path);

            let mut belonging_lib_online = false;
            let mut in_library = false;

            for lib_path in valid_library_paths {
                if f_path.starts_with(lib_path) {
                    in_library = true;
                    if std::path::Path::new(lib_path).exists() {
                        belonging_lib_online = true;
                    }
                    break;
                }
            }

            let mut keep = true;
            if !in_library {
                // 不屬於任何有設定的媒體庫路徑，應該刪除
                keep = false;
            } else if belonging_lib_online {
                // 媒體庫有掛載（在線），我們這才進行實體檔案檢查
                if v_path.try_exists().is_ok_and(|exists| !exists) {
                    keep = false;
                    println!(
                        "DEBUG: Pruning because v_path.exists() is false: {:?}",
                        v_path
                    );
                }
            } else {
                // 媒體庫沒掛載/不存在（外接硬碟拔除了），我們*保留*這個紀錄，不刪除
                keep = true;
            }

            if !keep {
                missing_ids.push(id);
            }
        }

        if missing_ids.is_empty() {
            return Ok(0);
        }

        // 刪除不存在或不在媒體庫範圍的記錄
        let mut deleted_count = 0;
        let conn = self.conn.lock().unwrap();
        for id in missing_ids {
            conn.execute("DELETE FROM videos WHERE id = ?1", params![id])?;
            deleted_count += 1;
        }

        Ok(deleted_count)
    }
}
