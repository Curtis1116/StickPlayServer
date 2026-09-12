use std::path::{Component, Path, PathBuf};

pub fn identifier(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("識別碼只能包含英數字、底線及連字號".into());
    }
    Ok(())
}

// Resolve existing ancestors too, so new files cannot escape through symlink parents.
pub fn resolve(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("必須使用不含 .. 的絕對路徑".into());
    }
    if path.symlink_metadata().is_ok() {
        return path.canonicalize().map_err(|_| "路徑無法解析".into());
    }
    let parent = path.parent().ok_or("路徑無效")?;
    Ok(resolve(parent)?.join(path.file_name().ok_or("路徑無效")?))
}

pub fn media_path(path: &Path) -> Result<PathBuf, String> {
    let root = std::env::var("STICKPLAY_MEDIA_DIR").unwrap_or_else(|_| "/media".into());
    let root = resolve(Path::new(&root))?;
    let resolved = resolve(path)?;
    let config = std::env::var("STICKPLAY_CONFIG_DIR").unwrap_or_else(|_| "./config".into());
    let config = std::fs::canonicalize(config).map_err(|_| "設定目錄不存在")?;
    if !resolved.starts_with(&root) || resolved.starts_with(config) {
        return Err("路徑不在允許的媒體目錄內".into());
    }
    Ok(resolved)
}

pub fn within(path: &Path, roots: &[String]) -> Result<PathBuf, String> {
    let path = media_path(path)?;
    if !roots
        .iter()
        .filter_map(|r| media_path(Path::new(r)).ok())
        .any(|r| path.starts_with(r))
    {
        return Err("路徑不在目前媒體庫內".into());
    }
    Ok(path)
}

pub fn extension(path: &Path, allowed: &[&str]) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !allowed.contains(&ext.as_str()) {
        return Err("不允許的檔案類型".into());
    }
    Ok(())
}
pub const VIDEOS: &[&str] = &[
    "mp4", "mkv", "avi", "wmv", "mov", "ts", "flv", "rmvb", "webm", "m4v",
];
pub const IMAGES: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp"];
