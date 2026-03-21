use serde::{Deserialize, Serialize};

/// 影片資料結構 — 從 DB 讀取後回傳給前端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEntry {
    pub id: String,
    pub title: String,
    pub actors: Vec<String>,
    pub genres: Vec<String>,
    pub level: String,
    pub rating: f64,
    pub criticrating: i32,
    pub release_date: String,
    pub date_added: String,
    pub video_path: String,
    pub folder_path: String,
    pub poster_path: Option<String>,
    pub nfo_path: Option<String>,  // 原始 .nfo（唯讀）
    pub nfos_path: Option<String>, // 程式專用 .nfos（可讀寫）
    pub is_favorite: bool,
}

/// 從 .nfo/.nfos 解析出的中繼資料
#[derive(Debug, Clone, Default)]
pub struct NfoData {
    pub num: Option<String>,
    pub title: String,
    pub level: Option<String>,
    pub is_uncensored: bool,
    pub actors: Vec<String>,
    pub genres: Vec<String>,
    pub rating: Option<f64>,
    pub criticrating: Option<i32>,
    pub release_date: String,
    pub date_added: String,
    pub poster: Option<String>,
}

/// 從資料夾名稱 Regex 解析出的中繼資料
#[derive(Debug, Clone)]
pub struct FolderMeta {
    pub id: String,
    pub actor: Option<String>,
    pub level: String,
    pub is_uncensored: bool,
}

/// 前端傳來的篩選/排序參數
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoFilter {
    pub genres: Option<Vec<String>>,
    pub levels: Option<Vec<String>>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub favorites_only: Option<bool>,
}

/// 媒體庫設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub id: String,
    pub name: String,
    pub paths: Vec<String>,
    pub db_name: String,
}
