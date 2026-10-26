use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::Hub;
use super::proxy::{Pass, authorize};
use crate::proto::Op;

const API: &str = "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&dt=t&dj=1";
pub const LYRICS_LIMIT: usize = 512 * 1024;
const MAX_LINES: usize = 400;
const BATCH: usize = 3000;
const KEEP: usize = 4096;
const FRESH: Duration = Duration::from_secs(7 * 86400);

// 訳し手を　一人置きては　覚えさす
#[derive(Clone)]
pub struct Translator(Arc<Mutex<Shelf>>, ureq::Agent);

#[derive(Default)]
struct Shelf {
    done: HashMap<String, (Arc<Value>, Instant)>,
}

impl Default for Translator {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_recv_response(Some(Duration::from_secs(20)))
            .build()
            .new_agent();
        Self(Arc::default(), agent)
    }
}

// 歌詞の行　刻を払いて　声ばかり
fn verses(source: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for raw in source.lines() {
        let mut rest = raw.trim();
        let mut stamped = false;
        while let Some(tag) = rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
            if !tag.0.starts_with(|c: char| c.is_ascii_digit()) {
                stamped = false;
                rest = "";
                break;
            }
            stamped = true;
            rest = tag.1;
        }
        let mut text = String::new();
        let mut skip = false;
        for c in rest.chars() {
            match c {
                '<' => skip = true,
                '>' if skip => skip = false,
                _ if !skip => text.push(c),
                _ => {}
            }
        }
        let text = text.trim().to_string();
        if (stamped || !raw.trim_start().starts_with('[')) && !text.is_empty() && !seen.contains(&text) {
            seen.push(text);
        }
        if seen.len() >= MAX_LINES {
            break;
        }
    }
    seen
}

// 束ね方　三千字ほどを　ひと包み
fn batches(lines: &[String]) -> Vec<&[String]> {
    let mut out = Vec::new();
    let (mut start, mut size) = (0, 0);
    for (i, l) in lines.iter().enumerate() {
        if size + l.len() > BATCH && i > start {
            out.push(&lines[start..i]);
            (start, size) = (i, 0);
        }
        size += l.len() + 1;
    }
    if start < lines.len() {
        out.push(&lines[start..]);
    }
    out
}

fn primary(lang: &str) -> &str {
    lang.split('-').next().unwrap_or(lang)
}

// 母の字で　既に書かれし　行は措く
fn native(line: &str, to: &str) -> bool {
    let has = |lo: char, hi: char| line.chars().any(|c| (lo..=hi).contains(&c));
    let (han, kana, hangul) = (has('\u{4e00}', '\u{9fff}'), has('\u{3040}', '\u{30ff}'), has('\u{ac00}', '\u{d7af}'));
    match primary(to) {
        "zh" => han && !kana && !hangul,
        "ja" => kana,
        "ko" => hangul,
        _ => false,
    }
}

impl Translator {
    fn cached(&self, key: &str) -> Option<Arc<Value>> {
        self.0.lock().unwrap().done.get(key).filter(|(_, at)| at.elapsed() < FRESH).map(|(v, _)| v.clone())
    }

    fn keep(&self, key: String, v: Arc<Value>) {
        let mut shelf = self.0.lock().unwrap();
        if shelf.done.len() >= KEEP {
            shelf.done.clear();
        }
        shelf.done.insert(key, (v, Instant::now()));
    }

    // 行を束ね　改行のまま　訳を請う
    fn render(&self, lines: &[String], to: &str) -> Result<Value> {
        let lines: Vec<String> = lines.iter().filter(|l| !native(l, to)).cloned().collect();
        let url = format!("{API}&tl={to}");
        let mut weight: HashMap<String, usize> = HashMap::new();
        let mut map = serde_json::Map::new();
        for chunk in batches(&lines) {
            let q = chunk.join("\n");
            let out: Value = self.1.post(&url).send_form([("q", q.as_str())])?.body_mut().read_json()?;
            let said = out["sentences"].as_array().ok_or_else(|| anyhow!("unexpected reply"))?;
            let whole: String = said.iter().filter_map(|s| s["trans"].as_str()).collect();
            let parts: Vec<&str> = whole.split('\n').collect();
            if let Some(lang) = out["src"].as_str() {
                *weight.entry(lang.to_string()).or_default() += q.chars().count();
            }
            if parts.len() != chunk.len() {
                continue;
            }
            for (src, text) in chunk.iter().zip(parts) {
                let text = text.trim();
                if !text.is_empty() && text != src.trim() {
                    map.insert(src.to_string(), Value::String(text.to_string()));
                }
            }
        }
        // 声の国　多きに従い　見定める
        let from = weight.into_iter().max_by_key(|(_, n)| *n).map(|(l, _)| l).unwrap_or_default();
        if primary(&from) == primary(to) {
            map.clear();
        }
        Ok(json!({ "from": from, "lines": map }))
    }
}


// 札を剥ぎ　歌の文だけ　取り出す
fn parts(raw: &str) -> (String, String) {
    let mut rest = raw.trim();
    let mut stamps = String::new();
    while let Some((tag, tail)) = rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
        if !tag.starts_with(|c: char| c.is_ascii_digit()) {
            break;
        }
        stamps.push('[');
        stamps.push_str(tag);
        stamps.push(']');
        rest = tail;
    }
    let mut text = String::new();
    let mut skip = false;
    for c in rest.chars() {
        match c {
            '<' => skip = true,
            '>' if skip => skip = false,
            _ if !skip => text.push(c),
            _ => {}
        }
    }
    (stamps, text.trim().to_string())
}

// 同じ刻に　母の言葉を　重ね置く
pub async fn bilingual(hub: &Hub, station: &str, track: &str, to: &str, source: &str) -> Result<String> {
    let found = maps(hub, station, track, to, source).await?;
    let map = found["lines"].as_object();
    let mut out = String::with_capacity(source.len() * 2);
    for raw in source.lines() {
        out.push_str(raw);
        out.push('\n');
        let (stamps, text) = parts(raw);
        if text.is_empty() {
            continue;
        }
        let Some(said) = map.and_then(|m| m.get(&text)).and_then(Value::as_str) else { continue };
        if said.is_empty() || said == text {
            continue;
        }
        if !stamps.is_empty() {
            out.push_str(&stamps);
        }
        out.push_str(said);
        out.push('\n');
    }
    Ok(out)
}

// 訳の棚　同じ歌なら　取り出だす
pub async fn maps(hub: &Hub, station: &str, track: &str, to: &str, source: &str) -> Result<Arc<Value>> {
    let key = format!("{station}\n{track}\n{to}");
    if let Some(v) = hub.translator.cached(&key) {
        return Ok(v);
    }
    let lines = verses(source);
    let tr = hub.translator.clone();
    let lang = to.to_string();
    let value = tokio::task::spawn_blocking(move || tr.render(&lines, &lang)).await??;
    let value = Arc::new(value);
    hub.translator.keep(key, value.clone());
    Ok(value)
}

// 異国の歌　隣の行に　母の声
pub async fn lyrics(
    State(hub): State<Hub>,
    Path((station, track, to)): Path<(String, String, String)>,
    Query(p): Query<Pass>,
) -> Response {
    if !(2..=10).contains(&to.len()) || !to.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-') {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let link = match authorize(&hub, &station, p.k.as_deref()) {
        Ok(l) => l,
        Err(code) => return code.into_response(),
    };
    let key = format!("{station}\n{track}\n{to}");
    let found = match hub.translator.cached(&key) {
        Some(v) => v,
        None => {
            let (head, body) = match link.request(Op::Lyrics { track }, None).await {
                Ok(r) => r,
                Err(code) => return code.into_response(),
            };
            if head.status != 200 {
                return StatusCode::NOT_FOUND.into_response();
            }
            let Some(bytes) = body.collect(LYRICS_LIMIT).await else { return StatusCode::BAD_GATEWAY.into_response() };
            let lines = verses(&String::from_utf8_lossy(&bytes));
            let tr = hub.translator.clone();
            let lang = to.clone();
            match tokio::task::spawn_blocking(move || tr.render(&lines, &lang)).await {
                Ok(Ok(v)) => {
                    let v = Arc::new(v);
                    hub.translator.keep(key, v.clone());
                    v
                }
                Ok(Err(err)) => {
                    tracing::warn!("translate: {err:#}");
                    return StatusCode::BAD_GATEWAY.into_response();
                }
                Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
            }
        }
    };
    ([(header::CACHE_CONTROL, "private, max-age=86400")], axum::Json(found.as_ref().clone())).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 刻と札　剥ぎ取りてなお　歌残る
    #[test]
    fn keeps_only_sung_lines() {
        let lrc = "[ti:Plastic Love]\n[00:01.00]<00:01.00>Hello <00:01.50>world\n[00:02.00][00:09.00]Again\n[00:03.00]\n[00:04.00]Again\nplain";
        assert_eq!(verses(lrc), ["Hello world", "Again", "plain"]);
        assert_eq!(primary("zh-CN"), "zh");
        assert!(native("但已是 能接受你所有改變", "zh-CN"));
        assert!(!native("君の名は", "zh-CN"));
        assert!(native("君の名は", "ja"));
        let many: Vec<String> = (0..10).map(|_| "x".repeat(1000)).collect();
        assert_eq!(batches(&many).iter().map(|b| b.len()).collect::<Vec<_>>(), [2, 2, 2, 2, 2]);
    }
}
