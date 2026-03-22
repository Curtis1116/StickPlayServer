use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::database::Database;
use crate::parser::{parse_folder_name, parse_nfo};

/// 影片副檔名清單
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "wmv", "mov", "ts", "flv", "rmvb"];

/// 圖片副檔名清單
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

/// 海報理想長寬比 (2:3)
const POSTER_RATIO: f64 = 2.0 / 3.0;

/// 長寬比距離低於此門檻視為「合適的海報」
const RATIO_THRESHOLD: f64 = 0.25;

/// 掃描媒體庫路徑，將結果寫入 SQLite
pub fn scan_library_paths(db: &Database, paths: &[String]) -> Result<usize, String> {
    if paths.is_empty() {
        crate::app_log!("[SCAN] 掃描路徑為空，跳過。");
        return Ok(0);
    }
    crate::app_log!("[SCAN] 開始大批掃描, 路徑集: {:?}", paths);

    let mut count = 0;

    for root_path in paths {
        let root = Path::new(root_path);
        if !root.exists() || !root.is_dir() {
            continue;
        }

        for entry in WalkDir::new(root)
            .min_depth(0)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }

            if scan_single_folder(db, entry.path(), false).is_ok() {
                count += 1;
            }
        }
    }

    match db.prune_missing_videos(paths) {
        Ok(deleted) if deleted > 0 => {
            crate::app_log!("已清除 {} 筆失效的影片記錄", deleted);
        }
        Err(e) => {
            crate::app_log!("清除失效影片記錄失敗: {}", e);
        }
        _ => {}
    }

    Ok(count)
}

/// 掃描單一資料夾，更新索引
/// 提示：此版本為「純讀取」模式，不會自動生成海報，也不會修改 .nfo
pub fn scan_single_folder(
    db: &Database,
    dir_path: &Path,
    _force_regen_poster: bool,
) -> Result<(), String> {
    // 尋找影片檔
    let video_path = find_video_file(dir_path).ok_or_else(|| "資料夾內無影片檔".to_string())?;

    let folder_name = dir_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let folder_meta = parse_folder_name(&folder_name);

    // 僅尋找 .nfo 檔案，不再支持 .nfos
    let nfo_path = find_file_by_ext(dir_path, "nfo");

    // 解析中繼資料
    let nfo_data = nfo_path
        .as_ref()
        .and_then(|p| parse_nfo(Path::new(p)).ok())
        .unwrap_or_default();

    let id = nfo_data.num.clone()
        .or_else(|| folder_meta.as_ref().map(|m| m.id.clone()))
        .unwrap_or_else(|| folder_name.clone());

    let level = nfo_data.level.clone()
        .or_else(|| folder_meta.as_ref().map(|m| m.level.clone()))
        .unwrap_or_default();

    let is_uncensored = nfo_data.is_uncensored || folder_meta.as_ref().map(|m| m.is_uncensored).unwrap_or(false);

    // 尋找海報圖（僅尋找現成圖片，不自動裁切）
    let poster_path = find_best_poster(dir_path, &video_path);

    // 產生縮圖
    if let Some(ref p) = poster_path {
        let thumbnail_dir = db.app_data_dir.join("thumbnails");
        if let Err(e) = std::fs::create_dir_all(&thumbnail_dir) {
            crate::app_log!("[THUMB] 無法建立縮圖目錄: {}, 錯誤: {}", thumbnail_dir.display(), e);
        } else {
            let safe_id = id.replace("/", "_").replace("\\", "_").replace(":", "_");
            let thumb_path = thumbnail_dir.join(format!("{}.jpg", safe_id));
            
            if !thumb_path.exists() {
                crate::app_log!("[THUMB] 正在為 [{}] 產生縮圖...", id);
                match std::fs::read(p) {
                    Ok(bytes) => {
                        match image::load_from_memory(&bytes) {
                            Ok(img) => {
                                let thumb = img.thumbnail(300, 450);
                                if let Err(e) = thumb.save(&thumb_path) {
                                    crate::app_log!("[THUMB] 儲存失敗 [{}]: {}", id, e);
                                } else {
                                    crate::app_log!("[THUMB] 成功產生 [{}]", id);
                                }
                            }
                            Err(e) => {
                                crate::app_log!("[THUMB] 無法解析圖片內容 [{}], 路徑: {}, 錯誤: {}", id, p, e);
                            }
                        }
                    }
                    Err(e) => {
                        crate::app_log!("[THUMB] 無法讀取實體檔案 [{}], 路徑: {}, 錯誤: {}", id, p, e);
                    }
                }
            } else {
                crate::app_log!("[THUMB] 跳過 [{}]: 縮圖已存在", id);
            }
        }
    } else {
        crate::app_log!("[THUMB] 跳過 [{}]: 找不到適合的海報 (poster_path 為空)", id);
    }

    let mut genres = Vec::new();
    let has_nfo_uncensored = nfo_data.genres.iter().any(|g| g == "無碼") || is_uncensored;
    if has_nfo_uncensored {
        genres.push("無碼".to_string());
    }

    let title = if nfo_data.title.is_empty() {
        folder_name.clone()
    } else {
        nfo_data.title.clone()
    };

    let mut final_actors = nfo_data.actors.clone();
    if final_actors.is_empty() {
        if let Some(meta) = &folder_meta {
            if let Some(actor) = &meta.actor {
                final_actors.push(actor.clone());
            }
        }
    }

    // 寫入資料庫 (包含 criticrating)
    db.upsert_video(
        &id,
        &title,
        &level,
        nfo_data.rating,
        &nfo_data.release_date,
        &nfo_data.date_added,
        &video_path,
        &dir_path.to_string_lossy(),
        poster_path.as_deref(),
        nfo_path.as_deref(),
        None, // 徹底拋棄 nfos_path
        &final_actors,
        &genres,
        nfo_data.criticrating.unwrap_or(0),
    )
    .map_err(|e| format!("寫入資料庫失敗: {}", e))?;

    Ok(())
}

fn find_video_file(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if VIDEO_EXTENSIONS.contains(&ext_lower.as_str()) {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn find_file_by_ext(dir: &Path, ext_target: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_string_lossy().to_lowercase() == ext_target {
                    return Some(path.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn collect_images(dir: &Path) -> Vec<PathBuf> {
    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
                        if stem != "stick_poster" {
                            images.push(path);
                        }
                    }
                }
            }
        }
    }
    images
}

/// 尋找最佳海報：此版本「僅尋找」現成圖片，不再進行自動生成
fn find_best_poster(dir: &Path, video_path: &str) -> Option<String> {
    let images = collect_images(dir);
    if images.is_empty() {
        return None;
    }

    // 1. 尋找 poster.jpg
    for img in &images {
        let stem = img.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
        if stem == "poster" {
            return Some(img.to_string_lossy().to_string());
        }
    }

    // 3. 尋找與影片同名的圖片
    let video_stem = Path::new(video_path).file_stem().and_then(|s| s.to_str()).map(|s| s.to_lowercase());
    for img in &images {
        let stem = img.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_lowercase();
        if let Some(ref vs) = video_stem {
            if stem == *vs {
                return Some(img.to_string_lossy().to_string());
            }
        }
    }

    // 4. 退而求其次，尋找比例最合適（最靠近 2:3）的現成圖片
    let mut scored: Vec<(f64, &PathBuf)> = Vec::new();
    for img in &images {
        if let Ok(dims) = image::image_dimensions(img) {
            let ratio = dims.0 as f64 / dims.1 as f64;
            let distance = (ratio - POSTER_RATIO).abs();
            scored.push((distance, img));
        }
    }
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((distance, best_img)) = scored.first() {
        if *distance < RATIO_THRESHOLD {
            return Some(best_img.to_string_lossy().to_string());
        }
    }

    // 5. 若無合適比例，直接回傳第一張圖片作為 fallback
    images.get(0).map(|p| p.to_string_lossy().to_string())
}
