use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::database::Database;
use crate::parser::{create_nfo_from_folder, parse_folder_name, parse_nfo};

/// 影片副檔名清單
const VIDEO_EXTENSIONS: &[&str] = crate::security::VIDEOS;

/// 圖片副檔名清單
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];

/// 海報理想長寬比 (2:3)
const POSTER_RATIO: f64 = 2.0 / 3.0;

/// 長寬比距離低於此門檻視為「合適的海報」
const RATIO_THRESHOLD: f64 = 0.25;

const THUMBNAIL_WIDTH: u32 = 300;
const THUMBNAIL_HEIGHT: u32 = 450;
const THUMBNAIL_WEBP_QUALITY: f32 = 72.0;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub indexed: usize,
    pub webp_thumbnails: usize,
    pub missing_thumbnails: usize,
    pub legacy_jpg_converted: usize,
    pub legacy_jpg_failed: usize,
    pub scan_errors: usize,
}

#[derive(Default)]
struct ThumbnailMigration {
    converted: usize,
    failed: usize,
}

pub(crate) fn thumbnail_safe_id(id: &str) -> String {
    id.replace("/", "_").replace("\\", "_").replace(":", "_")
}

/// 產生供媒體櫃使用的有損 WebP 縮圖。縮圖在掃描或裁切時預先建立，圖片請求只需讀檔。
pub(crate) fn save_thumbnail(image: &image::DynamicImage, path: &Path) -> Result<(), String> {
    let thumbnail = image.thumbnail(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT).to_rgb8();
    let encoded =
        webp::Encoder::from_rgb(thumbnail.as_raw(), thumbnail.width(), thumbnail.height())
            .encode(THUMBNAIL_WEBP_QUALITY);
    std::fs::write(path, &*encoded).map_err(|error| error.to_string())?;

    // WebP 寫入成功後才移除同一影片的舊 JPG 縮圖；媒體資料夾中的 poster.jpg
    // 是原始海報來源，不在此清理範圍內。
    let legacy_jpg = path.with_extension("jpg");
    if legacy_jpg != path && legacy_jpg.exists() {
        if let Err(error) = std::fs::remove_file(&legacy_jpg) {
            crate::app_log!("[THUMB] 舊 JPG 縮圖清理失敗 ({:?}): {}", legacy_jpg, error);
        }
    }
    Ok(())
}

/// 將舊版縮圖目錄中剩餘的 JPG 轉為 WebP。這也涵蓋原始海報已遺失、但舊縮圖仍
/// 可用的影片；成功轉換後才刪除 JPG。
fn migrate_legacy_thumbnails(db: &Database) -> ThumbnailMigration {
    let mut report = ThumbnailMigration::default();
    let thumbnail_dir = db.thumbnail_dir();
    let Ok(entries) = std::fs::read_dir(&thumbnail_dir) else {
        return report;
    };

    for entry in entries.flatten() {
        let jpg_path = entry.path();
        let is_jpg = jpg_path.is_file()
            && jpg_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg")
                });
        if !is_jpg {
            continue;
        }

        let webp_path = jpg_path.with_extension("webp");
        let result = if webp_path.is_file() && image::image_dimensions(&webp_path).is_ok() {
            std::fs::remove_file(&jpg_path).map_err(|error| error.to_string())
        } else {
            image::open(&jpg_path)
                .map_err(|error| error.to_string())
                .and_then(|image| save_thumbnail(&image, &webp_path))
                .and_then(|_| {
                    if jpg_path.exists() {
                        std::fs::remove_file(&jpg_path).map_err(|error| error.to_string())
                    } else {
                        Ok(())
                    }
                })
        };

        match result {
            Ok(()) => report.converted += 1,
            Err(error) => {
                report.failed += 1;
                crate::app_log!("[THUMB] 舊 JPG 縮圖轉換失敗 ({:?}): {}", jpg_path, error);
            }
        }
    }

    report
}

/// 掃描媒體庫路徑，將結果寫入 SQLite
/// 掃描期間會設定 `is_scanning` 旗標，防止 `switch_database` 在中途插隊
pub fn scan_library_paths(db: &Database, paths: &[String]) -> Result<ScanReport, String> {
    db.set_scanning(true);
    let result = scan_library_paths_inner(db, paths);
    db.set_scanning(false);
    result
}

fn scan_library_paths_inner(db: &Database, paths: &[String]) -> Result<ScanReport, String> {
    if paths.is_empty() {
        let deleted = db.prune_missing_videos(paths).map_err(|e| e.to_string())?;
        crate::app_log!("[SCAN] 掃描路徑為空，已清除 {} 筆索引。", deleted);
        return Ok(ScanReport {
            indexed: 0,
            webp_thumbnails: 0,
            missing_thumbnails: 0,
            legacy_jpg_converted: 0,
            legacy_jpg_failed: 0,
            scan_errors: 0,
        });
    }
    crate::app_log!("[SCAN] 開始大批掃描, 路徑集: {:?}", paths);

    let mut count = 0;
    let mut scan_errors = 0;

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

            match scan_single_folder(db, entry.path(), false) {
                Ok(()) => count += 1,
                Err(error) if error == "資料夾內無影片檔" => {}
                Err(error) => {
                    scan_errors += 1;
                    crate::app_log!("[SCAN] 掃描失敗 ({:?}): {}", entry.path(), error);
                }
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

    let migration = migrate_legacy_thumbnails(db);
    let videos = db
        .query_videos(&crate::models::VideoFilter::default())
        .map_err(|error| error.to_string())?;
    let expected_thumbnails = videos
        .iter()
        .filter(|video| video.poster_path.is_some())
        .collect::<Vec<_>>();
    let webp_thumbnails = expected_thumbnails
        .iter()
        .filter(|video| {
            db.resolve_thumbnail(&thumbnail_safe_id(&video.id))
                .is_some()
        })
        .count();
    let missing_thumbnails = expected_thumbnails.len().saturating_sub(webp_thumbnails);

    crate::app_log!(
        "[SCAN] 完成：索引 {}、WebP {}、缺少縮圖 {}、舊 JPG 轉換 {}、轉換失敗 {}、掃描錯誤 {}",
        count,
        webp_thumbnails,
        missing_thumbnails,
        migration.converted,
        migration.failed,
        scan_errors
    );

    Ok(ScanReport {
        indexed: count,
        webp_thumbnails,
        missing_thumbnails,
        legacy_jpg_converted: migration.converted,
        legacy_jpg_failed: migration.failed,
        scan_errors,
    })
}

/// 掃描單一資料夾，更新索引
pub fn scan_single_folder(
    db: &Database,
    dir_path: &Path,
    _force_regen_poster: bool,
) -> Result<(), String> {
    crate::security::media_path(dir_path)?;
    // 尋找影片檔
    let video_path = find_video_file(dir_path).ok_or_else(|| "資料夾內無影片檔".to_string())?;

    let folder_name = dir_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let folder_meta = parse_folder_name(&folder_name);

    // Create basic metadata only when no NFO is present.
    let mut nfo_path = find_file_by_ext(dir_path, "nfo");
    if nfo_path.is_none() {
        if let Some(meta) = &folder_meta {
            let generated_path = dir_path.join(format!("{}.nfo", meta.id));
            let date_added = chrono::Local::now().format("%Y-%m-%d").to_string();
            let create_result = crate::security::media_path(&generated_path)
                .and_then(|_| create_nfo_from_folder(&generated_path, meta, &date_added));
            match create_result {
                Ok(()) => {
                    crate::app_log!("[NFO] 已自動建立: {}", generated_path.display());
                    nfo_path = Some(generated_path.to_string_lossy().to_string());
                }
                Err(error) => crate::app_log!(
                    "[NFO] 自動建立失敗，仍繼續產生索引與縮圖 ({:?}): {}",
                    generated_path,
                    error
                ),
            }
        } else {
            crate::app_log!(
                "[NFO] 無法從資料夾名稱解析中繼資料，略過自動建立: {}",
                dir_path.display()
            );
        }
    }

    // 解析中繼資料
    let nfo_data = nfo_path
        .as_ref()
        .and_then(|p| parse_nfo(Path::new(p)).ok())
        .unwrap_or_default();

    let id = nfo_data
        .num
        .clone()
        .or_else(|| folder_meta.as_ref().map(|m| m.id.clone()))
        .unwrap_or_else(|| folder_name.clone());

    let level = nfo_data
        .level
        .clone()
        .or_else(|| folder_meta.as_ref().map(|m| m.level.clone()))
        .unwrap_or_default();

    let is_uncensored = nfo_data.uncensored_override.unwrap_or(
        nfo_data.is_uncensored
            || nfo_data.genres.iter().any(|g| g == "無碼")
            || folder_meta
                .as_ref()
                .map(|m| m.is_uncensored)
                .unwrap_or(false),
    );

    // 尋找海報圖
    let poster_path = find_best_poster(dir_path, &video_path);

    // 產生縮圖 (隔離路徑修復)
    if let Some(ref p) = poster_path {
        let thumbnail_dir = db.thumbnail_dir();
        let safe_id = thumbnail_safe_id(&id);
        let thumb_path = thumbnail_dir.join(format!("{}.webp", safe_id));

        if !thumb_path.exists() {
            crate::app_log!("[THUMB] 正在為 [{}] 產生縮圖...", id);
            match std::fs::read(p) {
                Ok(bytes) => match image::load_from_memory(&bytes) {
                    Ok(img) => {
                        if let Err(e) = save_thumbnail(&img, &thumb_path) {
                            crate::app_log!(
                                "[THUMB] [{}] 縮圖儲存失敗 ({:?}): {}",
                                id,
                                thumb_path,
                                e
                            );
                        }
                    }
                    Err(e) => {
                        crate::app_log!("[THUMB] [{}] 海報圖解碼失敗 ({:?}): {}", id, p, e);
                    }
                },
                Err(e) => {
                    crate::app_log!("[THUMB] [{}] 讀取海報圖失敗 ({:?}): {}", id, p, e);
                }
            }
        }

        // 已有 WebP 的媒體也順手清掉同目錄內的舊 JPG 縮圖。
        if thumb_path.exists() {
            let legacy_jpg = thumb_path.with_extension("jpg");
            if legacy_jpg.exists() {
                let _ = std::fs::remove_file(legacy_jpg);
            }
        }
    }

    let mut genres = Vec::new();
    for genre in &nfo_data.genres {
        let genre = genre.trim();
        if !genre.is_empty() && !genres.iter().any(|item| item == genre) {
            genres.push(genre.to_string());
        }
    }
    let has_nfo_uncensored = is_uncensored;
    if has_nfo_uncensored && !genres.iter().any(|genre| genre == "無碼") {
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

    // 寫入資料庫
    db.upsert_video(
        &id,
        &title,
        &level,
        nfo_data.rating,
        &nfo_data.year,
        &nfo_data.release_date,
        &nfo_data.date_added,
        &video_path,
        &dir_path.to_string_lossy(),
        poster_path.as_deref(),
        nfo_path.as_deref(),
        None,
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
        if path.is_file()
            && crate::security::media_path(&path).ok().is_some_and(|p| {
                dir.canonicalize()
                    .ok()
                    .is_some_and(|root| p.starts_with(root))
            })
        {
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
        if path.is_file()
            && crate::security::media_path(&path).ok().is_some_and(|p| {
                dir.canonicalize()
                    .ok()
                    .is_some_and(|root| p.starts_with(root))
            })
        {
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
            if path.is_file()
                && crate::security::media_path(&path).ok().is_some_and(|p| {
                    dir.canonicalize()
                        .ok()
                        .is_some_and(|root| p.starts_with(root))
                })
            {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default()
                            .to_lowercase();
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

    // 1. 尋找 poster.jpg（最優先）
    for img in &images {
        let stem = img
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if stem == "poster" {
            return Some(img.to_string_lossy().to_string());
        }
    }

    // 2. 尋找比例最合適（最靠近 2:3）且在門檻內的圖片
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

    // 3. 最後退而求其次，尋找與影片同名的圖片
    let video_stem = Path::new(video_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());
    for img in &images {
        let stem = img
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if let Some(ref vs) = video_stem {
            if stem == *vs {
                return Some(img.to_string_lossy().to_string());
            }
        }
    }

    // 4. 最終 fallback：直接回傳第一張圖片
    images.get(0).map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::{migrate_legacy_thumbnails, save_thumbnail};
    use crate::database::Database;

    #[test]
    fn thumbnail_is_webp_and_replaces_legacy_jpg() {
        let dir = tempfile::tempdir().unwrap();
        let webp_path = dir.path().join("movie.webp");
        let jpg_path = dir.path().join("movie.jpg");
        std::fs::write(&jpg_path, b"legacy thumbnail").unwrap();

        let source = image::DynamicImage::new_rgb8(600, 900);
        save_thumbnail(&source, &webp_path).unwrap();

        assert!(!jpg_path.exists());
        assert_eq!(
            image::ImageReader::open(&webp_path)
                .unwrap()
                .with_guessed_format()
                .unwrap()
                .format(),
            Some(image::ImageFormat::WebP)
        );
        assert_eq!(image::image_dimensions(webp_path).unwrap(), (300, 450));
    }

    #[test]
    fn library_refresh_migrates_remaining_jpg_thumbnails() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().to_path_buf(), "stickplay").unwrap();
        let jpg_path = db.thumbnail_dir().join("legacy-movie.jpg");
        image::DynamicImage::new_rgb8(300, 450)
            .save(&jpg_path)
            .unwrap();

        let report = migrate_legacy_thumbnails(&db);

        assert_eq!(report.converted, 1);
        assert_eq!(report.failed, 0);
        assert!(!jpg_path.exists());
        assert_eq!(
            image::image_dimensions(jpg_path.with_extension("webp")).unwrap(),
            (300, 450)
        );
    }
}
