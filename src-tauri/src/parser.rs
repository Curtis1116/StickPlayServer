use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

use crate::models::{FolderMeta, NfoData};

/// 從 .nfo XML 檔案解析中繼資料
pub fn parse_nfo(nfo_path: &Path) -> Result<NfoData, String> {
    let content =
        std::fs::read_to_string(nfo_path).map_err(|e| format!("讀取 .nfo 失敗: {}", e))?;

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
                    "releasedate" | "premiered" | "release_date" => {
                        data.release_date = text;
                    }
                    "dateadded" | "date_added" => {
                        data.date_added = text;
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

    Ok(data)
}

/// 從資料夾名稱解析 ID、演員與等級
/// 格式: XXX-XXXX (AAAA_BB) 或 XXX-XXXX (AAAA_BBX)
/// 也支援包含前綴或數字: [2024-01-01] 300MIUM-1311 (明日香_S)
/// 若括號內無 _ 則視為全為演員: NPV-030 (NULL)
pub fn parse_folder_name(name: &str) -> Option<FolderMeta> {
    // 移除 ^ 錨點以容許前綴，並使用 [A-Za-z0-9]+ 支援如 300MIUM 包含數字的片商
    let re = Regex::new(r"([A-Za-z0-9]+-\d+)\s*\(([^)]+)\)").ok()?;
    let caps = re.captures(name)?;

    let id = caps.get(1)?.as_str().to_uppercase();
    let inner = caps.get(2)?.as_str().trim();

    let mut actor = None;
    let mut level_raw = "";

    // 尋找最後一個 _ 作為分隔點
    if let Some(last_underscore_idx) = inner.rfind('_') {
        let current_actor = &inner[..last_underscore_idx].trim();
        if !current_actor.eq_ignore_ascii_case("NULL") && !current_actor.is_empty() {
            actor = Some(current_actor.to_string());
        }
        level_raw = &inner[last_underscore_idx + 1..].trim();
    } else {
        // 如果沒有 _ ，整段視為演員
        if !inner.eq_ignore_ascii_case("NULL") && !inner.is_empty() {
            actor = Some(inner.to_string());
        }
    }

    // 判斷是否有 X 後綴表示「無碼」
    let is_uncensored = level_raw.ends_with('X') || level_raw.ends_with('x');

    // 移除 X 後綴取得純等級
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
/// 更新 .nfos 檔案（程式專用，永不修改原始 .nfo）
/// 寫入 <num> (ID) 及 <rating> 標籤
/// 若 .nfos 不存在，會從原始 .nfo 複製內容後修改，或建立新檔案
pub fn update_nfos(
    nfos_path: &Path,
    video_id: &str,
    rating: f64,
    original_nfo_path: Option<&str>,
) -> Result<(), String> {
    let rating_str = format!("{:.1}", rating);

    // 取得 .nfos 的基底內容
    let content = if nfos_path.exists() {
        std::fs::read_to_string(nfos_path).map_err(|e| format!("讀取 .nfos 失敗: {}", e))?
    } else if let Some(nfo_path) = original_nfo_path {
        let nfo = Path::new(nfo_path);
        if nfo.exists() {
            std::fs::read_to_string(nfo).map_err(|e| format!("讀取原始 .nfo 失敗: {}", e))?
        } else {
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<movie>\n    <num>{}</num>\n    <rating>{}</rating>\n</movie>\n",
                video_id, rating_str
            )
        }
    } else {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<movie>\n    <num>{}</num>\n    <rating>{}</rating>\n</movie>\n",
            video_id, rating_str
        )
    };

    // 處理 <rating>
    let rating_re =
        Regex::new(r"<rating>[^<]*</rating>").map_err(|e| format!("Regex 錯誤: {}", e))?;
    let content = if rating_re.is_match(&content) {
        rating_re
            .replace(&content, format!("<rating>{}</rating>", rating_str))
            .to_string()
    } else {
        let close_re = Regex::new(r"(</(?:movie|video|episodedetails|tvshow)>)")
            .map_err(|e| format!("Regex 錯誤: {}", e))?;
        if close_re.is_match(&content) {
            close_re
                .replace(&content, format!("    <rating>{}</rating>\n$1", rating_str))
                .to_string()
        } else {
            format!("{}\n<rating>{}</rating>\n", content.trim_end(), rating_str)
        }
    };

    // 處理 <num> (ID)
    let num_re = Regex::new(r"<num>[^<]*</num>").map_err(|e| format!("Regex 錯誤: {}", e))?;
    let content = if num_re.is_match(&content) {
        num_re
            .replace(&content, format!("<num>{}</num>", video_id))
            .to_string()
    } else {
        let close_re = Regex::new(r"(</(?:movie|video|episodedetails|tvshow)>)")
            .map_err(|e| format!("Regex 錯誤: {}", e))?;
        if close_re.is_match(&content) {
            close_re
                .replace(&content, format!("    <num>{}</num>\n$1", video_id))
                .to_string()
        } else {
            format!("{}\n<num>{}</num>\n", content.trim_end(), video_id)
        }
    };

    std::fs::write(nfos_path, content).map_err(|e| format!("寫入 .nfos 失敗: {}", e))?;

    Ok(())
}

/// 更新 .nfos 檔案中的多個標籤
pub fn update_nfos_full(
    nfos_path: &Path,
    video_id: &str,
    rating: f64,
    level: &str,
    actors: &[String],
    release_date: &str,
    date_added: &str,
    is_uncensored: bool,
    original_nfo_path: Option<&str>,
) -> Result<(), String> {
    let rating_str = format!("{:.1}", rating);

    // 取得 .nfos 的基底內容
    let mut content = if nfos_path.exists() {
        std::fs::read_to_string(nfos_path).map_err(|e| format!("讀取 .nfos 失敗: {}", e))?
    } else if let Some(nfo_path) = original_nfo_path {
        let nfo = Path::new(nfo_path);
        if nfo.exists() {
            std::fs::read_to_string(nfo).map_err(|e| format!("讀取原始 .nfo 失敗: {}", e))?
        } else {
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<movie>\n</movie>\n".to_string()
        }
    } else {
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<movie>\n</movie>\n".to_string()
    };

    // Helper closure: 替換或新增單一標籤
    let update_tag = |xml: &mut String, tag: &str, value: &str| {
        let re = Regex::new(&format!(r"<{tag}>[^<]*</{tag}>")).unwrap();
        if re.is_match(xml) {
            // 如果已有該標籤，則替換所有匹配項目
            if tag != "actor" && tag != "genre" && tag != "tag" {
                *xml = re
                    .replace(xml, format!("<{}>{}</{}>", tag, value, tag))
                    .to_string();
            }
        } else {
            // 如果沒有，則加在結尾標籤之前
            let close_re = Regex::new(r"(</(?:movie|video|episodedetails|tvshow)>)").unwrap();
            if close_re.is_match(xml) {
                *xml = close_re
                    .replace(xml, format!("    <{}>{}</{}>\n$1", tag, value, tag))
                    .to_string();
            } else {
                *xml = format!("{}\n<{}>{}</{}>\n", xml.trim_end(), tag, value, tag);
            }
        }
    };

    update_tag(&mut content, "num", video_id);
    update_tag(&mut content, "rating", &rating_str);
    update_tag(&mut content, "releasedate", release_date);
    update_tag(&mut content, "dateadded", date_added);
    update_tag(&mut content, "level", &format!("{}{}", level, if is_uncensored { "X" } else { "" }));

    // 處理演員 (先移除所有舊的 actor 標籤)
    let actor_re = Regex::new(r"(?s)<actor>.*?</actor>\s*").unwrap();
    content = actor_re.replace_all(&content, "").to_string();

    // 加入新的 actor 標籤
    let mut actors_xml = String::new();
    for actor in actors {
        if !actor.trim().is_empty() {
            actors_xml.push_str(&format!(
                "    <actor>\n        <name>{}</name>\n    </actor>\n",
                actor.trim()
            ));
        }
    }

    // 處理 genre/tag
    // 這裡我們保留現有的非 "無碼" genre，然後重寫
    // 我們使用簡易的方法：先把所有 `<genre>...` 刪除，再補上原本除了無碼以外的，加上我們想設定的
    let genre_re = Regex::new(r"(?s)<genre>.*?</genre>\s*").unwrap();
    let mut existing_genres = Vec::new();
    for cap in genre_re.find_iter(&content.clone()) {
        let text = cap.as_str();
        if !text.contains("無碼") {
            existing_genres.push(text.to_string());
        }
    }
    content = genre_re.replace_all(&content, "").to_string();

    let tag_re = Regex::new(r"(?s)<tag>.*?</tag>\s*").unwrap();
    let mut existing_tags = Vec::new();
    for cap in tag_re.find_iter(&content.clone()) {
        let text = cap.as_str();
        if !text.contains("無碼") {
            existing_tags.push(text.to_string());
        }
    }
    content = tag_re.replace_all(&content, "").to_string();

    let mut genres_tags_xml = String::new();
    for g in existing_genres {
        genres_tags_xml.push_str(&g);
    }
    for t in existing_tags {
        genres_tags_xml.push_str(&t);
    }

    if is_uncensored {
        genres_tags_xml.push_str("    <genre>無碼</genre>\n    <tag>無碼</tag>\n");
    }

    let close_re = Regex::new(r"(</(?:movie|video|episodedetails|tvshow)>)").unwrap();
    if close_re.is_match(&content) {
        content = close_re
            .replace(&content, format!("{}{}$1", actors_xml, genres_tags_xml))
            .to_string();
    } else {
        content.push_str(&actors_xml);
        content.push_str(&genres_tags_xml);
    }

    std::fs::write(nfos_path, content).map_err(|e| format!("寫入 .nfos 失敗: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_folder_name_basic() {
        let result = parse_folder_name("ABC-123 (SomeSeries_SA)").unwrap();
        assert_eq!(result.id, "ABC-123");
        assert_eq!(result.actor.unwrap(), "SomeSeries");
        assert_eq!(result.level, "SA");
        assert!(!result.is_uncensored);
    }

    #[test]
    fn test_parse_folder_name_uncensored() {
        let result = parse_folder_name("MIDE-258 (Series_SAX)").unwrap();
        assert_eq!(result.id, "MIDE-258");
        assert_eq!(result.actor.unwrap(), "Series");
        assert_eq!(result.level, "SA");
        assert!(result.is_uncensored);
    }

    #[test]
    fn test_parse_folder_name_level_ss() {
        let result = parse_folder_name("ABF-317 (Actress_SS)").unwrap();
        assert_eq!(result.id, "ABF-317");
        assert_eq!(result.actor.unwrap(), "Actress");
        assert_eq!(result.level, "SS");
        assert!(!result.is_uncensored);
    }

    #[test]
    fn test_parse_folder_name_numerical_and_prefix() {
        let result =
            parse_folder_name(r"\\192.168.1.86\share\[Preview]\300MIUM-1311 (明日香_S)").unwrap();
        assert_eq!(result.id, "300MIUM-1311");
        assert_eq!(result.actor.unwrap(), "明日香");
        assert_eq!(result.level, "S");
        assert!(!result.is_uncensored);

        let result2 = parse_folder_name(r"259LUXU-1430 (辻井ほのか_SS)").unwrap();
        assert_eq!(result2.id, "259LUXU-1430");
        assert_eq!(result2.actor.unwrap(), "辻井ほのか");
        assert_eq!(result2.level, "SS");
    }

    #[test]
    fn test_parse_folder_name_null_actor_no_level() {
        let result = parse_folder_name(r"[Selection]\NPV-030 (NULL)").unwrap();
        assert_eq!(result.id, "NPV-030");
        assert_eq!(result.actor, None);
        assert_eq!(result.level, "");
        assert!(!result.is_uncensored);
    }

    #[test]
    fn test_parse_folder_name_no_match() {
        let result = parse_folder_name("SomeRandomFolder");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_nfo_basic() {
        let temp_dir = std::env::temp_dir().join("stickplay_test_nfo");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let nfo_path = temp_dir.join("test.nfo");
        std::fs::write(
            &nfo_path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<movie>
    <title>Test Movie</title>
    <rating>8.5</rating>
    <releasedate>2024-01-15</releasedate>
    <dateadded>2024-02-01</dateadded>
    <genre>Action</genre>
    <genre>Drama</genre>
    <actor>
        <name>Actor One</name>
    </actor>
    <actor>
        <name>Actor Two</name>
    </actor>
</movie>"#,
        )
        .unwrap();

        let data = parse_nfo(&nfo_path).unwrap();
        assert_eq!(data.title, "Test Movie");
        assert_eq!(data.rating, Some(8.5));
        assert_eq!(data.actors, vec!["Actor One", "Actor Two"]);
        assert_eq!(data.genres, vec!["Action", "Drama"]);

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_update_nfos_existing() {
        let temp_dir = std::env::temp_dir().join("stickplay_test_nfos_rating");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let nfos_path = temp_dir.join("test.nfos");
        std::fs::write(
            &nfos_path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<movie>
    <title>Test</title>
    <rating>5.0</rating>
</movie>"#,
        )
        .unwrap();

        update_nfos(&nfos_path, "ABC-123", 9.3, None).unwrap();
        let content = std::fs::read_to_string(&nfos_path).unwrap();
        assert!(content.contains("<rating>9.3</rating>"));
        assert!(content.contains("<num>ABC-123</num>"));

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_update_nfos_from_nfo() {
        let temp_dir = std::env::temp_dir().join("stickplay_test_nfos_from_nfo");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let nfo_path = temp_dir.join("test.nfo");
        let nfos_path = temp_dir.join("test.nfos");
        std::fs::write(
            &nfo_path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<movie>
    <title>Test</title>
</movie>"#,
        )
        .unwrap();

        update_nfos(&nfos_path, "XYZ-456", 7.5, Some(nfo_path.to_str().unwrap())).unwrap();
        let nfos_content = std::fs::read_to_string(&nfos_path).unwrap();
        assert!(nfos_content.contains("<rating>7.5</rating>"));
        assert!(nfos_content.contains("<num>XYZ-456</num>"));
        assert!(nfos_content.contains("<title>Test</title>"));

        // 原始 .nfo 不應被修改
        let nfo_content = std::fs::read_to_string(&nfo_path).unwrap();
        assert!(!nfo_content.contains("<rating>"));
        assert!(!nfo_content.contains("<num>"));

        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_update_nfos_no_source() {
        let temp_dir = std::env::temp_dir().join("stickplay_test_nfos_new");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let nfos_path = temp_dir.join("test.nfos");

        update_nfos(&nfos_path, "DEF-789", 8.0, None).unwrap();
        let content = std::fs::read_to_string(&nfos_path).unwrap();
        assert!(content.contains("<rating>8.0</rating>"));
        assert!(content.contains("<num>DEF-789</num>"));

        std::fs::remove_dir_all(temp_dir).ok();
    }
}
