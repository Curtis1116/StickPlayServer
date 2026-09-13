use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

use crate::models::{FolderMeta, NfoData};

/// 從 .nfo XML 檔案解析中繼資料
pub fn parse_nfo(nfo_path: &Path) -> Result<NfoData, String> {
    let mut content =
        std::fs::read_to_string(nfo_path).map_err(|e| format!("讀取 .nfo 失敗: {}", e))?;

    // 移除 UTF-8 BOM (\u{feff}) 並去除開頭空白
    content = content
        .trim_start_matches('\u{feff}')
        .trim_start()
        .to_string();

    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut data = NfoData::default();
    let mut current_tag = String::new();
    let mut in_actor = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match tag_name.as_str() {
                    "actor" => in_actor = true,
                    "level" => {
                        data.level = Some(String::new());
                        current_tag = tag_name;
                    }
                    _ => current_tag = tag_name,
                }
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"level" => {
                data.level = Some(String::new());
            }
            Ok(Event::End(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag_name == "actor" {
                    in_actor = false;
                }
                current_tag.clear();
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if text.is_empty() {
                    continue;
                }
                match current_tag.as_str() {
                    "uncensored" => data.uncensored_override = Some(text == "true"),
                    "num" => data.num = Some(text),
                    "title" if !in_actor => data.title = text,
                    "level" => {
                        if text.to_uppercase().ends_with('X') {
                            data.is_uncensored = true;
                            data.level = Some(text[..text.len() - 1].to_string());
                        } else {
                            data.level = Some(text);
                        }
                    }
                    "name" if in_actor => {
                        if !text.is_empty() {
                            data.actors.push(text);
                        }
                    }
                    "genre" => {
                        if !text.is_empty() {
                            data.genres.push(text);
                        }
                    }
                    "tag" if text == "無碼" => data.is_uncensored = true,
                    "year" => data.year = text,
                    "rating" | "userrating" => {
                        if let Ok(r) = text.parse::<f64>() {
                            data.rating = Some(r);
                        }
                    }
                    "criticrating" => {
                        if let Ok(cr) = text.parse::<i32>() {
                            data.criticrating = Some(cr);
                        }
                    }
                    "releasedate" | "premiered" | "release_date" => {
                        data.release_date = text;
                    }
                    "dateadded" | "date_added" => {
                        data.date_added = text;
                    }
                    "poster" | "thumb" => {
                        data.poster = Some(text);
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML 解析錯誤: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    // 將 <criticrating> 與 <rating> 同步
    match (data.criticrating, data.rating) {
        (Some(cr), _) => {
            // 以 criticrating 為準
            data.rating = Some(cr as f64 / 10.0);
        }
        (None, Some(r)) => {
            // 缺少 criticrating，自動生成
            data.criticrating = Some((r * 10.0).round() as i32);
        }
        _ => {}
    }

    if let Some(value) = data.uncensored_override {
        data.is_uncensored = value;
    }
    Ok(data)
}

/// 從資料夾名稱解析 ID、演員與等級
pub fn parse_folder_name(name: &str) -> Option<FolderMeta> {
    let re = Regex::new(r"([A-Za-z0-9]+-\d+)\s*\(([^)]+)\)").ok()?;
    let caps = re.captures(name)?;

    let id = caps.get(1)?.as_str().to_uppercase();
    let inner = caps.get(2)?.as_str().trim();

    let mut actor = None;
    let mut level_raw = "";

    if let Some(last_underscore_idx) = inner.rfind('_') {
        let current_actor = &inner[..last_underscore_idx].trim();
        if !current_actor.eq_ignore_ascii_case("NULL") && !current_actor.is_empty() {
            actor = Some(current_actor.to_string());
        }
        level_raw = &inner[last_underscore_idx + 1..].trim();
    } else {
        if !inner.eq_ignore_ascii_case("NULL") && !inner.is_empty() {
            actor = Some(inner.to_string());
        }
    }

    let is_uncensored = level_raw.ends_with('X') || level_raw.ends_with('x');
    let level = if is_uncensored {
        &level_raw[..level_raw.len() - 1]
    } else {
        level_raw
    };

    Some(FolderMeta {
        id,
        actor,
        level: level.to_uppercase(),
        is_uncensored,
    })
}

/// Replace only requested direct children. Preserve comments, nested metadata,
/// repeated actors, and compact XML without depending on whitespace or regexes.
fn update_nfo_surgical(
    nfo_path: &Path,
    tags: Vec<(String, String)>,
    is_uncensored: Option<bool>,
) -> Result<(), String> {
    let content = if nfo_path.exists() {
        std::fs::read_to_string(nfo_path).map_err(|e| e.to_string())?
    } else {
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><movie></movie>".into()
    };
    let mut reader = Reader::from_str(&content);
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut name = String::new();
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut close = None;
    let mut root_seen = false;
    let should_replace = |name: &str, xml: &str| {
        tags.iter().any(|(tag, _)| tag == name)
            || (tags.iter().any(|(tag, _)| tag == "actor") && name == "actor")
            || (tags.iter().any(|(tag, _)| tag == "releasedate")
                && ["premiered", "release_date"].contains(&name))
            || (tags.iter().any(|(tag, _)| tag == "dateadded") && name == "date_added")
            || (tags.iter().any(|(tag, _)| tag == "rating") && name == "userrating")
            || (is_uncensored.is_some()
                && ["genre", "tag"].contains(&name)
                && xml
                    .split_once('>')
                    .and_then(|(_, v)| v.split_once('<'))
                    .is_some_and(|(v, _)| v.trim() == "無碼"))
    };
    loop {
        let before = reader.buffer_position() as usize;
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Start(e) => {
                if depth == 0 {
                    if root_seen {
                        return Err("NFO 根節點重複".into());
                    }
                    root_seen = true;
                }
                if depth == 1 {
                    start = before;
                    name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                }
                depth += 1;
            }
            Event::Empty(e) if depth == 1 => {
                let end = reader.buffer_position() as usize;
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if should_replace(&name, &content[before..end]) {
                    edits.push((before, end, String::new()));
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or("NFO 結構無效")?;
                if depth == 1 {
                    let end = reader.buffer_position() as usize;
                    if should_replace(&name, &content[start..end]) {
                        edits.push((start, end, String::new()));
                    }
                }
                if depth == 0 {
                    close = Some(before);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 {
        return Err("NFO 結構不完整".into());
    }
    let close = close.ok_or("NFO 缺少根節點")?;
    let mut block = String::new();
    for (_, xml) in tags {
        block.push_str(&xml);
        block.push('\n');
    }
    if is_uncensored == Some(true) {
        block.push_str("<genre>無碼</genre><tag>無碼</tag>\n");
    }
    edits.push((close, close, block));
    edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut output = content;
    for (start, end, replacement) in edits {
        output.replace_range(start..end, &replacement);
    }
    let temp = nfo_path.with_file_name(format!(".stickplay-{}.tmp", crate::auth::token()));
    std::fs::write(&temp, output).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, nfo_path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        e.to_string()
    })
}

pub fn update_nfo(
    nfo_path: &Path,
    video_id: &str,
    rating: f64,
    critic_rating_opt: Option<i32>,
    date_added: &str,
) -> Result<(), String> {
    let video_id = quick_xml::escape::escape(video_id);
    let date_added = quick_xml::escape::escape(date_added);
    let mut tags = Vec::new();
    let synchronized_critic_rating = critic_rating_opt.unwrap_or_else(|| {
        if rating > 0.0 {
            (rating * 10.0).round() as i32
        } else {
            0
        }
    });
    let synchronized_rating = synchronized_critic_rating as f64 / 10.0;

    // 保留原本的 dateadded，避免手術式更新時被 MANAGED_TAGS 清除後遺失
    if !date_added.is_empty() {
        tags.push((
            "dateadded".to_string(),
            format!("<dateadded>{}</dateadded>", date_added),
        ));
    }
    tags.push(("num".to_string(), format!("<num>{}</num>", video_id)));
    tags.push((
        "rating".to_string(),
        format!("<rating>{:.1}</rating>", synchronized_rating),
    ));
    tags.push((
        "criticrating".to_string(),
        format!(
            "<criticrating>{}</criticrating>",
            synchronized_critic_rating
        ),
    ));

    if let Some(parent) = nfo_path.parent() {
        if parent.join("poster.jpg").exists() {
            tags.push((
                "poster".to_string(),
                "<poster>poster.jpg</poster>".to_string(),
            ));
        }
    }

    tags.push((
        "lockdata".to_string(),
        "<lockdata>true</lockdata>".to_string(),
    ));

    update_nfo_surgical(nfo_path, tags, None)
}

pub fn update_nfo_full(
    nfo_path: &Path,
    video_id: &str,
    rating: f64,
    critic_rating_opt: Option<i32>,
    actors: &[String],
    release_date: &str,
    date_added: &str,
    is_uncensored: bool,
    title: &str,
    level: &str,
    year: &str,
    genres: &[String],
) -> Result<(), String> {
    let mut tags = Vec::new();

    let title_str = quick_xml::escape::escape(title);
    let video_id = quick_xml::escape::escape(video_id);
    let release_date = quick_xml::escape::escape(release_date);
    let date_added = quick_xml::escape::escape(date_added);
    let level = quick_xml::escape::escape(level);
    let year = quick_xml::escape::escape(year);
    tags.push(("level".into(), format!("<level>{level}</level>")));
    tags.push(("year".into(), format!("<year>{year}</year>")));
    tags.push((
        "uncensored".into(),
        format!("<uncensored>{is_uncensored}</uncensored>"),
    ));
    // An empty replacement removes all existing actor nodes too.
    tags.push(("actor".into(), String::new()));
    // The marker removes all existing genre nodes before the edited values are appended.
    tags.push(("genre".into(), String::new()));
    for genre in genres {
        let genre = genre.trim();
        if !genre.is_empty() && genre != "無碼" {
            tags.push((
                "genre".into(),
                format!("<genre>{}</genre>", quick_xml::escape::escape(genre)),
            ));
        }
    }

    let synchronized_critic_rating = critic_rating_opt.unwrap_or_else(|| {
        if rating > 0.0 {
            (rating * 10.0).round() as i32
        } else {
            0
        }
    });
    let synchronized_rating = synchronized_critic_rating as f64 / 10.0;

    tags.push((
        "lockdata".to_string(),
        "<lockdata>true</lockdata>".to_string(),
    ));
    tags.push((
        "dateadded".to_string(),
        format!("<dateadded>{}</dateadded>", date_added),
    ));
    tags.push(("title".to_string(), format!("<title>{}</title>", title_str)));
    tags.push((
        "sorttitle".to_string(),
        format!("<sorttitle>{}</sorttitle>", title_str),
    ));
    tags.push(("num".to_string(), format!("<num>{}</num>", video_id)));
    tags.push((
        "rating".to_string(),
        format!("<rating>{:.1}</rating>", synchronized_rating),
    ));
    tags.push((
        "criticrating".to_string(),
        format!(
            "<criticrating>{}</criticrating>",
            synchronized_critic_rating
        ),
    ));

    if let Some(parent) = nfo_path.parent() {
        if parent.join("poster.jpg").exists() {
            tags.push((
                "poster".to_string(),
                "<poster>poster.jpg</poster>".to_string(),
            ));
        }
    }

    tags.push((
        "releasedate".to_string(),
        format!("<releasedate>{}</releasedate>", release_date),
    ));

    for actor in actors {
        if !actor.trim().is_empty() {
            tags.push((
                "actor".to_string(),
                format!(
                    "<actor>\n  <name>{}</name>\n  <type>Actor</type>\n</actor>",
                    quick_xml::escape::escape(actor.trim())
                ),
            ));
        }
    }

    update_nfo_surgical(nfo_path, tags, Some(is_uncensored))
}

/// Cropping changes only the poster reference, never the title, rating or tags.
pub fn update_poster_nfo(path: &Path) -> Result<(), String> {
    update_nfo_surgical(
        path,
        vec![("poster".into(), "<poster>poster.jpg</poster>".into())],
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_and_full_updates_preserve_unrelated_xml() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("movie.nfo");
        std::fs::write(&path,"<movie><title>Keep me</title><actor><name>A</name></actor><actor><name>B</name></actor><genre>無碼</genre><fileinfo><title>Nested</title></fileinfo></movie>").unwrap();
        update_poster_nfo(&path).unwrap();
        let first = std::fs::read_to_string(&path).unwrap();
        assert!(first.contains("<title>Keep me</title>"));
        assert!(first.contains("<name>B</name>"));
        assert!(first.contains("<genre>無碼</genre>"));
        update_nfo_full(
            &path,
            "ID-1",
            8.0,
            Some(80),
            &["A & B".into(), "C <D>".into()],
            "2026-01-01",
            "2026-09-10",
            false,
            "A & title",
            "",
            "2026",
            &["劇情".into(), "精選".into()],
        )
        .unwrap();
        let data = parse_nfo(&path).unwrap();
        assert_eq!(data.actors, vec!["A & B", "C <D>"]);
        assert_eq!(data.title, "A & title");
        assert_eq!(data.level, Some("".into()));
        assert_eq!(data.year, "2026");
        assert_eq!(data.genres, vec!["劇情", "精選"]);
        assert_eq!(data.uncensored_override, Some(false));
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("<fileinfo><title>Nested</title></fileinfo>"));
        update_nfo_full(
            &path,
            "ID-1",
            8.0,
            Some(80),
            &[],
            "",
            "",
            false,
            "",
            "",
            "",
            &[],
        )
        .unwrap();
        assert!(parse_nfo(&path).unwrap().actors.is_empty());
    }
    #[test]
    fn invalid_xml_does_not_overwrite_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.nfo");
        let original = "<movie><title>Broken</movie>";
        std::fs::write(&path, original).unwrap();
        assert!(update_poster_nfo(&path).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }
}
