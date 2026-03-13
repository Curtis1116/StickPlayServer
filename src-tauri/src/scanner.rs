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
/// 回傳掃描到的影片數量
pub fn scan_library_paths(db: &Database, paths: &[String]) -> Result<usize, String> {
    let mut count = 0;

    for root_path in paths {
        let root = Path::new(root_path);
        if !root.exists() || !root.is_dir() {
            continue;
        }

        // 遞迴走訪目錄，包含根目錄
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

    // 掃描完成後，清除資料庫中對應資料夾已不存在，或不在媒體庫範圍內的記錄
    match db.prune_missing_videos(paths) {
        Ok(deleted) if deleted > 0 => {
            println!("已清除 {} 筆失效的影片記錄", deleted);
        }
        Err(e) => {
            eprintln!("清除失效影片記錄失敗: {}", e);
        }
        _ => {}
    }

    Ok(count)
}

/// 掃描單一資料夾，更新索引及海報
/// `force_regen_poster`: 若為 true，會先刪除舊的 stick_poster 再重新生成
pub fn scan_single_folder(
    db: &Database,
    dir_path: &Path,
    force_regen_poster: bool,
) -> Result<(), String> {
    // 尋找該資料夾內的影片檔
    let video_path = find_video_file(dir_path).ok_or_else(|| "資料夾內無影片檔".to_string())?;

    // 取得資料夾名稱
    let folder_name = dir_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    // 解析資料夾命名規則 (作為 fallback)
    let folder_meta = parse_folder_name(&folder_name);

    // 尋找 .nfos（優先）和 .nfo 檔案
    let nfos_path = find_file_by_ext(dir_path, "nfos");
    let nfo_path = find_file_by_ext(dir_path, "nfo");

    // 解析中繼資料：優先 .nfos → 回退 .nfo（唯讀）
    let nfo_data = nfos_path
        .as_ref()
        .and_then(|p| parse_nfo(Path::new(p)).ok())
        .or_else(|| nfo_path.as_ref().and_then(|p| parse_nfo(Path::new(p)).ok()))
        .unwrap_or_default();

    // 優先權：NFO > 資料夾解析 (folder_meta)
    let id = nfo_data.num.clone()
        .or_else(|| folder_meta.as_ref().map(|m| m.id.clone()))
        .unwrap_or_else(|| folder_name.clone());

    let level = nfo_data.level.clone()
        .or_else(|| folder_meta.as_ref().map(|m| m.level.clone()))
        .unwrap_or_default();

    let is_uncensored = nfo_data.is_uncensored || folder_meta.as_ref().map(|m| m.is_uncensored).unwrap_or(false);

    // 如果強制重新生成海報，先刪除舊的 stick_poster
    if force_regen_poster {
        let old_poster = dir_path.join("stick_poster.jpg");
        if old_poster.exists() {
            // NAS 或檔案總管可能有暫時的 file-lock，加入重試機制
            for _ in 0..5 {
                if std::fs::remove_file(&old_poster).is_ok() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    }

    // 尋找海報圖（智慧選取 + 自動裁切）
    let poster_path = find_best_poster(dir_path, &video_path, force_regen_poster);

    // 組合 genres
    let mut genres = Vec::new();
    let has_nfo_uncensored = nfo_data.genres.iter().any(|g| g == "無碼") || is_uncensored;
    if has_nfo_uncensored {
        genres.push("無碼".to_string());
    }

    // 決定 title
    let title = if nfo_data.title.is_empty() {
        folder_name.clone()
    } else {
        nfo_data.title.clone()
    };

    // 決定 actors
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
        &nfo_data.release_date,
        &nfo_data.date_added,
        &video_path,
        &dir_path.to_string_lossy(),
        poster_path.as_deref(),
        nfo_path.as_deref(),
        nfos_path.as_deref(),
        &final_actors,
        &genres,
    )
    .map_err(|e| format!("寫入資料庫失敗: {}", e))?;

    Ok(())
}

/// 在指定目錄中尋找第一個影片檔案
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

/// 在指定目錄中尋找指定副檔名的檔案
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

/// 收集目錄內所有圖片路徑
fn collect_images(dir: &Path) -> Vec<PathBuf> {
    let mut images = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
                        images.push(path);
                    }
                }
            }
        }
    }
    images
}

/// 智慧海報選取：
/// 1. 已有 stick_poster → 除非 force_regen 為 true，否則直接使用
/// 2. 檔名含 poster 或與影片同名 → 使用
/// 3. 多張圖片 → 選長寬比最接近 2:3 的
/// 4. 最佳圖片長寬比仍不夠接近 → 自動裁切生成 stick_poster
fn find_best_poster(dir: &Path, video_path: &str, force_regen: bool) -> Option<String> {
    let images = collect_images(dir);
    if images.is_empty() {
        return None;
    }

    // 1. 已有 stick_poster 直接回傳 (若非強制重新產生)
    if !force_regen {
        for img in &images {
            let stem = img
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if stem == "stick_poster" {
                return Some(img.to_string_lossy().to_string());
            }
        }
    }

    // 將 stick_poster 從候選名單中排除 (因為要重新尋找原始圖)
    let candidate_images: Vec<PathBuf> = images
        .into_iter()
        .filter(|img| {
            let stem = img
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            stem != "stick_poster"
        })
        .collect();

    if candidate_images.is_empty() {
        return None;
    }

    // 2. 檔名含 poster 的
    for img in &candidate_images {
        let stem = img
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if stem.contains("poster") {
            // 檢查比例是否合適
            if let Ok(dims) = image::image_dimensions(img) {
                let ratio = dims.0 as f64 / dims.1 as f64;
                if (ratio - POSTER_RATIO).abs() < RATIO_THRESHOLD {
                    return Some(img.to_string_lossy().to_string());
                }
            } else {
                return Some(img.to_string_lossy().to_string());
            }
        }
    }

    // 3. 與影片同名的圖片
    let video_stem = Path::new(video_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase());
    for img in &candidate_images {
        let stem = img
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if let Some(ref vs) = video_stem {
            if &stem == vs {
                if let Ok(dims) = image::image_dimensions(img) {
                    let ratio = dims.0 as f64 / dims.1 as f64;
                    if (ratio - POSTER_RATIO).abs() < RATIO_THRESHOLD {
                        return Some(img.to_string_lossy().to_string());
                    }
                } else {
                    return Some(img.to_string_lossy().to_string());
                }
            }
        }
    }

    // 4. 所有圖片依長寬比接近 2:3 排序
    let mut scored: Vec<(f64, &PathBuf)> = Vec::new();
    for img in &candidate_images {
        if let Ok(dims) = image::image_dimensions(img) {
            let ratio = dims.0 as f64 / dims.1 as f64;
            let distance = (ratio - POSTER_RATIO).abs();
            scored.push((distance, img));
        }
    }
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((best_distance, best_img)) = scored.first() {
        if *best_distance < RATIO_THRESHOLD {
            // 比例夠接近，直接使用
            return Some(best_img.to_string_lossy().to_string());
        }
        // 比例不夠接近 → 自動裁切生成 stick_poster
        if let Some(cropped) = generate_stick_poster(dir, best_img) {
            return Some(cropped);
        }
        // 裁切失敗也回傳原圖
        return Some(best_img.to_string_lossy().to_string());
    }

    None
}

/// 自動裁切生成 stick_poster.jpg
/// 策略：使用原圖全高，裁切寬度 = 高 × 2/3，
/// 透過計算「最大連續垂直膚色」來尋找最完整的人像，並加入右側加權以優先抓取最右邊的人像（如封面右側的正面照）
fn generate_stick_poster(dir: &Path, source: &Path) -> Option<String> {
    use image::GenericImageView;

    let img = image::open(source).ok()?;
    let (width, height) = img.dimensions();

    // 計算目標裁切寬度（維持原圖高度，寬度 = 高 × 2/3）
    let target_width = ((height as f64) * POSTER_RATIO) as u32;

    if target_width >= width {
        // 原圖已經夠窄，不需裁切
        return None;
    }

    // 以連續垂直膚色找最佳裁切位置
    let crop_x = find_crop_by_continuous_skin(&img, width, height, target_width);

    // 裁切
    let cropped = img.crop_imm(crop_x, 0, target_width, height);

    // 儲存
    let output_path = dir.join("stick_poster.jpg");

    // NAS 或檔案系統可能有暫時的 file-lock，加入重試機制
    let mut saved = false;
    for _ in 0..5 {
        if cropped.save(&output_path).is_ok() {
            saved = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    if !saved {
        eprintln!("無法覆寫海報 {:?}", output_path);
        return None;
    }

    Some(output_path.to_string_lossy().to_string())
}

/// 尋找最佳裁切位置：
/// 針對包含「大張主視覺人像」與「多張小預覽圖」的 DVD 封面設計。
/// 尋找最佳裁切位置：
/// 優先使用人臉辨識 (Face Detection)。
/// 依據使用者需求：「由右至左掃描人臉占比最高的區塊」。
/// 若完全沒有偵測到人臉，則退回使用「網格率 (Grid Ratio)」演算法以避開背面小圖。
fn find_crop_by_continuous_skin(
    img: &image::DynamicImage,
    width: u32,
    height: u32,
    target_width: u32,
) -> u32 {
    let max_start_x = width.saturating_sub(target_width);
    if max_start_x == 0 {
        return 0;
    }

    // --- 1. Face Detection Strategy ---
    // 載入模型 (利用 include_bytes! 直接包裝進執行檔)
    let model_bytes = include_bytes!("../assets/seeta_fd_frontal_v1.0.bin");
    let mut reader = std::io::Cursor::new(&model_bytes[..]);

    if let Ok(model) = rustface::model::read_model(&mut reader) {
        let mut detector = rustface::create_detector_with_model(model);
        detector.set_min_face_size(20);
        detector.set_score_thresh(2.0);
        detector.set_pyramid_scale_factor(0.8);
        detector.set_slide_window_step(4, 4);

        // rustface 必須使用灰階圖
        let gray_img = img.to_luma8();
        let mut image_data = rustface::ImageData::new(gray_img.as_raw(), width, height);

        // 執行偵測
        let faces = detector.detect(&mut image_data);

        if !faces.is_empty() {
            use image::GenericImageView;
            // 預先計算每個直行的邊緣與梯度能量，加速視窗滑動評估
            let mut col_energy = vec![0u64; width as usize];
            let mut col_straight_edges = vec![0u64; width as usize];

            for y in 1..(height - 1) {
                for x in 1..(width - 1) {
                    let p_l = img.get_pixel(x - 1, y);
                    let p_r = img.get_pixel(x + 1, y);
                    let p_u = img.get_pixel(x, y - 1);
                    let p_d = img.get_pixel(x, y + 1);

                    let lum = |p: image::Rgba<u8>| {
                        (p[0] as i32 * 299 + p[1] as i32 * 587 + p[2] as i32 * 114) / 1000
                    };

                    let l = lum(p_l);
                    let r = lum(p_r);
                    let u = lum(p_u);
                    let d = lum(p_d);

                    let gx = (r - l).abs();
                    let gy = (d - u).abs();

                    let total_gradient = gx + gy;
                    let is_straight_edge = (gx > 30 && gy < 10) || (gy > 30 && gx < 10);

                    col_energy[x as usize] += total_gradient as u64;
                    if is_straight_edge {
                        col_straight_edges[x as usize] += 1;
                    }
                }
            }

            let step_x = std::cmp::max(1, width / 40);
            let mut best_start_x = max_start_x;
            let mut min_grid_ratio = f64::MAX;
            let mut valid_face_found = false;

            // 針對每一張潛在人臉，找出符合其範圍的最小網格率視窗
            for face in &faces {
                let bbox = face.bbox();
                let f_x = bbox.x().max(0) as u32;
                let f_w = bbox.width().max(0) as u32;

                // 確保候選的裁切視窗百分之百包含這張人臉
                let min_bound = if f_x + f_w > target_width {
                    (f_x + f_w) - target_width
                } else {
                    0
                };
                let max_bound = f_x.min(max_start_x);

                if min_bound <= max_bound {
                    valid_face_found = true;
                    // 評估所有合法的候選視窗
                    for start_x in (min_bound..=max_bound).step_by(step_x as usize) {
                        let mut window_energy = 0u64;
                        let mut window_straight_edges = 0u64;

                        let end_x = (start_x + target_width).min(width);
                        for x in start_x..end_x {
                            window_energy += col_energy[x as usize];
                            window_straight_edges += col_straight_edges[x as usize];
                        }

                        let energy = std::cmp::max(1, window_energy) as f64;
                        let ratio = (window_straight_edges as f64) / energy;

                        // 取網格率最低的視窗（過濾掉背面高雜訊的假人臉，只保留乾淨的正面）
                        if ratio < min_grid_ratio {
                            min_grid_ratio = ratio;
                            best_start_x = start_x;
                        }
                    }
                }
            }

            if valid_face_found {
                return best_start_x;
            }
        }
    }

    // --- 2. Fallback: Grid Ratio Strategy ---
    use image::GenericImageView;
    let mid_x = width / 2;
    let mut straight_edges_l = 0u64;
    let mut straight_edges_r = 0u64;
    let mut energy_l = 0u64;
    let mut energy_r = 0u64;

    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let p_c = img.get_pixel(x, y);
            let p_l = img.get_pixel(x - 1, y);
            let p_r = img.get_pixel(x + 1, y);
            let p_u = img.get_pixel(x, y - 1);
            let p_d = img.get_pixel(x, y + 1);

            let lum = |p: image::Rgba<u8>| {
                (p[0] as i32 * 299 + p[1] as i32 * 587 + p[2] as i32 * 114) / 1000
            };

            let c = lum(p_c);
            let l = lum(p_l);
            let r = lum(p_r);
            let u = lum(p_u);
            let d = lum(p_d);

            let gx = (r - l).abs();
            let gy = (d - u).abs();

            let total_gradient = gx + gy;
            let is_straight_edge = (gx > 30 && gy < 10) || (gy > 30 && gx < 10);

            if x < mid_x {
                energy_l += total_gradient as u64;
                if is_straight_edge {
                    straight_edges_l += 1;
                }
            } else {
                energy_r += total_gradient as u64;
                if is_straight_edge {
                    straight_edges_r += 1;
                }
            }
        }
    }

    let energy_l = std::cmp::max(1, energy_l) as f64;
    let energy_r = std::cmp::max(1, energy_r) as f64;

    let ratio_l = (straight_edges_l as f64) / energy_l;
    let ratio_r = (straight_edges_r as f64) / energy_r;

    if ratio_l < ratio_r {
        0
    } else {
        max_start_x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_crop_logic() {
        use std::io::Write;

        let output_path = r"C:\Users\huach\source\repos\StickPlay\crop_test_output.txt";
        let mut out = std::fs::File::create(output_path).expect("無法建立輸出檔案");

        let test_cases = vec![
            (
                r#"\\192.168.1.86\This is A Book\Classic Book\[Actors]\[Retired]\松本梨穂\390JNT-087 (松本梨穂_A)"#,
                "LEFT",
            ),
            (
                r#"\\192.168.1.86\This is A Book\Classic Book\[Actors]\[Retired]\希島あいり\IPZ-505 (希島あいり_A)"#,
                "RIGHT",
            ),
            (
                r#"\\192.168.1.86\This is A Book\Classic Book\[Actors]\[Retired]\上原結衣\XV-873 (上原結衣_S)"#,
                "RIGHT",
            ),
        ];

        let debug_dir = PathBuf::from(r"C:\Users\huach\source\repos\StickPlay");

        for (dir_path, expected_side) in test_cases {
            let path = PathBuf::from(dir_path);
            let images = collect_images(&path);

            // 列出所有找到的圖片
            writeln!(out, "\n=== Dir: {} ===", dir_path).ok();
            writeln!(out, "Found {} images:", images.len()).ok();
            for img in &images {
                let dims = image::image_dimensions(img);
                writeln!(
                    out,
                    "  {:?} => {:?}",
                    img.file_name().unwrap_or_default(),
                    dims
                )
                .ok();
            }

            // 選擇最寬的圖片（排除 stick_poster 和 debug_poster）
            let target_img = images
                .into_iter()
                .filter(|p| {
                    let stem = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    !stem.contains("stick_poster") && !stem.contains("debug_poster")
                })
                .max_by_key(|p| image::image_dimensions(p).map(|(w, _)| w).unwrap_or(0));

            if let Some(img_path) = target_img {
                writeln!(
                    out,
                    "Selected: {:?}",
                    img_path.file_name().unwrap_or_default()
                )
                .ok();
                if let Ok(img) = image::open(&img_path) {
                    use image::GenericImageView;
                    let (width, height) = img.dimensions();
                    let target_width = ((height as f64) * POSTER_RATIO) as u32;

                    if target_width >= width {
                        writeln!(
                            out,
                            "  SKIP: Image already narrow enough ({}x{}, target={})",
                            width, height, target_width
                        )
                        .ok();
                        continue;
                    }

                    // Calculate Grid Ratio (Straight Edges / Total Gradient Energy)
                    let mid_x = width / 2;
                    let mut straight_edges_l = 0u64;
                    let mut straight_edges_r = 0u64;
                    let mut energy_l = 0u64;
                    let mut energy_r = 0u64;

                    for y in 1..(height - 1) {
                        for x in 1..(width - 1) {
                            let p_c = img.get_pixel(x, y);
                            let p_l = img.get_pixel(x - 1, y);
                            let p_r = img.get_pixel(x + 1, y);
                            let p_u = img.get_pixel(x, y - 1);
                            let p_d = img.get_pixel(x, y + 1);

                            let lum = |p: image::Rgba<u8>| {
                                (p[0] as i32 * 299 + p[1] as i32 * 587 + p[2] as i32 * 114) / 1000
                            };

                            let c = lum(p_c);
                            let l = lum(p_l);
                            let r = lum(p_r);
                            let u = lum(p_u);
                            let d = lum(p_d);

                            let gx = (r - l).abs();
                            let gy = (d - u).abs();

                            let total_gradient = gx + gy;
                            let is_straight_edge = (gx > 30 && gy < 10) || (gy > 30 && gx < 10);

                            if x < mid_x {
                                energy_l += total_gradient as u64;
                                if is_straight_edge {
                                    straight_edges_l += 1;
                                }
                            } else {
                                energy_r += total_gradient as u64;
                                if is_straight_edge {
                                    straight_edges_r += 1;
                                }
                            }
                        }
                    }

                    // Avoid division by zero
                    let energy_l = std::cmp::max(1, energy_l) as f64;
                    let energy_r = std::cmp::max(1, energy_r) as f64;
                    let ratio_l = (straight_edges_l as f64) / energy_l;
                    let ratio_r = (straight_edges_r as f64) / energy_r;

                    writeln!(
                        out,
                        "  Ratio Left:  {:.5} (E: {}, SE: {})",
                        ratio_l, energy_l, straight_edges_l
                    )
                    .ok();
                    writeln!(
                        out,
                        "  Ratio Right: {:.5} (E: {}, SE: {})",
                        ratio_r, energy_r, straight_edges_r
                    )
                    .ok();

                    // Evaluate our test heuristic: The side with the lower Grid Ratio is the Front Cover!
                    let heuristic_side = if ratio_l < ratio_r { "LEFT" } else { "RIGHT" };
                    let crop_x = if heuristic_side == "LEFT" {
                        0
                    } else {
                        width.saturating_sub(target_width)
                    };

                    let max_start_x = width.saturating_sub(target_width);
                    let ratio = crop_x as f64 / max_start_x as f64;
                    let side = if ratio < 0.33 {
                        "LEFT"
                    } else if ratio > 0.66 {
                        "RIGHT"
                    } else {
                        "CENTER"
                    };

                    writeln!(
                        out,
                        "  Width: {}, Height: {}, Target Width: {}",
                        width, height, target_width
                    )
                    .ok();
                    writeln!(
                        out,
                        "  Crop X: {} / Max X: {} (ratio: {:.2})",
                        crop_x, max_start_x, ratio
                    )
                    .ok();
                    writeln!(
                        out,
                        "  Result Side: {} | Expected Side: {} | {}",
                        side,
                        expected_side,
                        if side == expected_side {
                            "✅ PASS"
                        } else {
                            "❌ FAIL"
                        }
                    )
                    .ok();

                    // 儲存 debug 截圖到本機
                    let folder_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let debug_output = debug_dir.join(format!("debug_crop_{}.jpg", folder_name));
                    let cropped = img.crop_imm(crop_x, 0, target_width, height);
                    if let Err(e) = cropped.save(&debug_output) {
                        writeln!(out, "  Failed to save debug image: {}", e).ok();
                    } else {
                        writeln!(out, "  Saved debug crop to: {:?}", debug_output).ok();
                    }
                } else {
                    writeln!(out, "  Failed to open image").ok();
                }
            } else {
                writeln!(out, "  No valid source image found").ok();
            }
        }

        writeln!(out, "\n=== DONE ===").ok();
        println!("Test output written to: {}", output_path);
    }
}
