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
    content = content.trim_start_matches('\u{feff}').trim_start().to_string();

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
                    _ => current_tag = tag_name,
                }
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
                    "genre" | "tag" => {
                        if !text.is_empty() {
                            data.genres.push(text);
                        }
                    }
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

const MANAGED_TAGS: &[&str] = &[
    "lockdata",
    "dateadded",
    "title",
    "actor",
    "rating",
    "criticrating",
    "poster",
    "thumb",
    "sorttitle",
    "releasedate",
    "num",
];

/// 手術式更新 NFO：僅移除頂層的受管標籤，並在末尾插入新資料，確保不破壞 fileinfo 等結構
fn update_nfo_surgical(
    nfo_path: &Path,
    new_managed_xml: Vec<(String, String)>,
    is_uncensored: bool,
) -> Result<(), String> {
    let mut content = if nfo_path.exists() {
        std::fs::read_to_string(nfo_path).map_err(|e| format!("讀取 .nfo 失敗: {}", e))?
    } else {
        "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>\n<movie>\n</movie>\n".to_string()
    };
    
    content = content.trim_start_matches('\u{feff}').trim_start().to_string();

    for name in MANAGED_TAGS {
        let re = Regex::new(&format!(r"(?ism)^\s*<{}[^>]*>.*?</{}>\s*", name, name)).unwrap();
        content = re.replace_all(&content, "").to_string();
    }

    let g_re = Regex::new(r"(?ism)^\s*<genre>無碼</genre>\s*").unwrap();
    content = g_re.replace_all(&content, "").to_string();
    let t_re = Regex::new(r"(?ism)^\s*<tag>無碼</tag>\s*").unwrap();
    content = t_re.replace_all(&content, "").to_string();

    let mut block = String::new();
    if !content.trim_end().is_empty() && !content.ends_with('\n') {
        block.push('\n');
    }

    for name in MANAGED_TAGS {
        if let Some((_, xml)) = new_managed_xml.iter().find(|(n, _)| n == name) {
            for line in xml.lines() {
                block.push_str("  ");
                block.push_str(line.trim());
                block.push('\n');
            }
        }
    }

    if is_uncensored {
        block.push_str("  <genre>無碼</genre>\n");
        block.push_str("  <tag>無碼</tag>\n");
    }

    let close_re = Regex::new(r"(?i)</(movie|video|episodedetails|tvshow)>").unwrap();
    if let Some(mat) = close_re.find_iter(&content).last() {
        content.insert_str(mat.start(), &block);
    } else {
        content.push_str(&block);
    }

    if !content.trim_end().ends_with('>') {
         content.push_str("\n</movie>\n");
    }

    std::fs::write(nfo_path, &content).map_err(|e| format!("寫入 .nfo 失敗: {}", e))?;
    Ok(())
}

pub fn update_nfo(
    nfo_path: &Path,
    video_id: &str,
    rating: f64,
    critic_rating_opt: Option<i32>,
) -> Result<(), String> {
    let mut tags = Vec::new();
    let synchronized_critic_rating = critic_rating_opt.unwrap_or_else(|| {
        if rating > 0.0 { (rating * 10.0).round() as i32 } else { 0 }
    });
    let synchronized_rating = synchronized_critic_rating as f64 / 10.0;

    tags.push(("num".to_string(), format!("<num>{}</num>", video_id)));
    tags.push(("rating".to_string(), format!("<rating>{:.1}</rating>", synchronized_rating)));
    tags.push(("criticrating".to_string(), format!("<criticrating>{}</criticrating>", synchronized_critic_rating)));
    
    if let Some(parent) = nfo_path.parent() {
        if parent.join("poster.jpg").exists() {
             tags.push(("poster".to_string(), "<poster>poster.jpg</poster>".to_string()));
        }
    }

    tags.push(("lockdata".to_string(), "<lockdata>true</lockdata>".to_string()));

    update_nfo_surgical(nfo_path, tags, false)
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
) -> Result<(), String> {
    let mut tags = Vec::new();

    let actors_joined = actors.iter().filter(|a| !a.trim().is_empty()).cloned().collect::<Vec<_>>().join(",");
    let title_str = if actors_joined.is_empty() {
        video_id.to_string()
    } else {
        format!("{}_({})", actors_joined, video_id)
    };

    let synchronized_critic_rating = critic_rating_opt.unwrap_or_else(|| {
        if rating > 0.0 { (rating * 10.0).round() as i32 } else { 0 }
    });
    let synchronized_rating = synchronized_critic_rating as f64 / 10.0;

    tags.push(("lockdata".to_string(), "<lockdata>true</lockdata>".to_string()));
    tags.push(("dateadded".to_string(), format!("<dateadded>{}</dateadded>", date_added)));
    tags.push(("title".to_string(), format!("<title>{}</title>", title_str)));
    tags.push(("sorttitle".to_string(), format!("<sorttitle>{}</sorttitle>", title_str)));
    tags.push(("num".to_string(), format!("<num>{}</num>", video_id)));
    tags.push(("rating".to_string(), format!("<rating>{:.1}</rating>", synchronized_rating)));
    tags.push(("criticrating".to_string(), format!("<criticrating>{}</criticrating>", synchronized_critic_rating)));

    if let Some(parent) = nfo_path.parent() {
        if parent.join("poster.jpg").exists() {
             tags.push(("poster".to_string(), "<poster>poster.jpg</poster>".to_string()));
        }
    }

    tags.push(("releasedate".to_string(), format!("<releasedate>{}</releasedate>", release_date)));

    for actor in actors {
        if !actor.trim().is_empty() {
            tags.push(("actor".to_string(), format!(
                "<actor>\n  <name>{}</name>\n  <type>Actor</type>\n</actor>",
                actor.trim()
            )));
        }
    }

    update_nfo_surgical(nfo_path, tags, is_uncensored)
}
