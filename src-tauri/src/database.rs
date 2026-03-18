use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::models::{VideoEntry, VideoFilter};

/// 資料庫封裝 — 使用 Mutex 確保線程安全
pub struct Database {
    pub conn: Mutex<Connection>,
    pub app_data_dir: PathBuf,
}

impl Database {
    /// 初始化資料庫連線及建立資料表
    pub fn new(app_data_dir: PathBuf) -> SqlResult<Self> {
        std::fs::create_dir_all(&app_data_dir).ok();

        let conn = Self::create_connection(&app_data_dir, "stickplay")?;

        Ok(Self {
            conn: Mutex::new(conn),
            app_data_dir,
        })
    }

    /// 建立與特定資料庫的連線，並確保資料表存在
    fn create_connection(app_data_dir: &PathBuf, db_name: &str) -> SqlResult<Connection> {
        let db_path = app_data_dir.join(format!("{}.db", db_name));
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
        conn.execute_batch("ALTER TABLE videos ADD COLUMN criticrating INTEGER NOT NULL DEFAULT 0;")
            .ok();

        // Migration: 清除除了「無碼」以外的舊分類標籤
        conn.execute("DELETE FROM video_genres WHERE genre != '無碼'", params![])
            .ok();

        Ok(conn)
    }

    /// 切換使用中的資料庫檔案（供多媒體庫功能使用）
    pub fn switch_database(&self, db_name: &str) -> SqlResult<()> {
        let new_conn = Self::create_connection(&self.app_data_dir, db_name)?;
        let mut conn_guard = self.conn.lock().unwrap();
        *conn_guard = new_conn;
        Ok(())
    }

    /// 插入或更新影片記錄
    pub fn upsert_video(
        &self,
        id: &str,
        title: &str,
        level: &str,
        rating: Option<f64>,
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
        let conn = self.conn.lock().unwrap();

        // 清除相同資料夾但 ID 不同的舊記錄 (避免修改 ID 擷取邏輯後產生重複記錄)
        conn.execute(
            "DELETE FROM videos WHERE folder_path = ?1 AND id != ?2",
            params![folder_path, id],
        )?;

        conn.execute(
            "INSERT INTO videos (id, title, level, rating, release_date, date_added, video_path, folder_path, poster_path, nfo_path, nfos_path, criticrating)
             VALUES (?1, ?2, ?3, COALESCE(?4, 0.0), ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                level = excluded.level,
                rating = COALESCE(excluded.rating, videos.rating),
                release_date = excluded.release_date,
                date_added = excluded.date_added,
                video_path = excluded.video_path,
                folder_path = excluded.folder_path,
                poster_path = excluded.poster_path,
                nfo_path = excluded.nfo_path,
                nfos_path = excluded.nfos_path,
                criticrating = excluded.criticrating",
            params![id, title, level, rating, release_date, date_added, video_path, folder_path, poster_path, nfo_path, nfos_path, criticrating],
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
            "SELECT v.id, v.title, v.level, v.rating, v.release_date, v.date_added,
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
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, bool>(11)?,
                row.get::<_, i32>(12)?,
            ))
        })?;

        let mut videos = Vec::new();
        for row in video_rows {
            let (
                id,
                title,
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

            // 【修復】檢查影片實體檔案是否還存在於檔案系統
            let mut keep = v_path.exists();
            if !keep {
                println!("DEBUG: Pruning because v_path.exists() is false: {:?}", v_path);
            }

            // 檢查資料夾是否屬於目前設定的媒體庫路徑之一
            if keep {
                let mut in_library = false;
                for lib_path in valid_library_paths {
                    if f_path.starts_with(lib_path) {
                        in_library = true;
                        break;
                    }
                }
                if !in_library {
                    keep = false;
                }
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
