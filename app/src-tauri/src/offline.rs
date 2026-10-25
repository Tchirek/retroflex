use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use ureq::ResponseExt;

const AUDIO: &[&str] = &["mp3", "m4a", "flac", "ogg", "opus", "wav", "aac", "aiff", "ape", "wv"];

// 持ち帰る　歌の置き場は　手の届く
pub fn shelf<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    #[cfg(desktop)]
    let base = app.path().audio_dir().map(|d| d.join("Retroflex")).or_else(|_| app.path().app_data_dir().map(|d| d.join("tracks")));
    #[cfg(mobile)]
    let base = app.path().app_data_dir().map(|d| d.join("tracks"));
    let dir = base.map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

// 名の末の　括弧の中に　札を読む
fn key_of(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    if !AUDIO.contains(&ext) {
        return None;
    }
    let inner = stem.rsplit_once('[')?.1.strip_suffix(']')?;
    let (station, id) = inner.split_once('_')?;
    let fine = |s: &str, extra: char| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == extra);
    (fine(station, '-') && fine(id, '0')).then(|| format!("{station}/{id}"))
}

// 禁じ字は　下線に替えて　名を整う
fn clean(name: &str) -> String {
    let s: String = name.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    let s = s.trim().trim_end_matches('.');
    let mut out: String = s.chars().take(120).collect();
    if out.is_empty() {
        out.push_str("track");
    }
    out
}

fn ext_for(mime: &str) -> Option<&'static str> {
    Some(match mime.split(';').next()?.trim() {
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/mp4" | "audio/x-m4a" | "audio/m4a" => "m4a",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/ogg" | "application/ogg" => "ogg",
        "audio/opus" => "opus",
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
        "audio/aac" => "aac",
        _ => return None,
    })
}

#[derive(Serialize)]
pub struct Kept {
    key: String,
    path: String,
}

// 棚の歌　ひとつひとつの　札を見る
#[tauri::command]
pub fn kept<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Kept>, String> {
    let dir = shelf(&app)?;
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    Ok(entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            key_of(&name).map(|key| Kept { key, path: e.path().to_string_lossy().into_owned() })
        })
        .collect())
}

#[derive(Clone, Serialize)]
struct Progress {
    key: String,
    got: u64,
    total: u64,
}

// 遠き歌　手元の棚へ　汲み移す
#[tauri::command]
pub async fn keep<R: Runtime>(app: AppHandle<R>, url: String, key: String, name: String, hint: Option<String>) -> Result<String, String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("bad url".into());
    }
    tauri::async_runtime::spawn_blocking(move || fetch(&app, &url, &key, &name, hint.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

fn fetch<R: Runtime>(app: &AppHandle<R>, url: &str, key: &str, name: &str, hint: Option<&str>) -> Result<String, String> {
    let dir = shelf(app)?;
    let agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .build()
        .new_agent();
    let mut resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mime = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let from_url = resp.get_uri().path().rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    let ext = ext_for(&mime)
        .map(String::from)
        .or(from_url.filter(|e| AUDIO.contains(&e.as_str())))
        .or(hint.filter(|e| AUDIO.contains(e)).map(String::from))
        .unwrap_or_else(|| "m4a".into());
    let total = resp.body().content_length().unwrap_or(0);
    let file = dir.join(format!("{}.{ext}", clean(name)));
    if key_of(&file.file_name().unwrap_or_default().to_string_lossy()).as_deref() != Some(key) {
        return Err("bad name".into());
    }
    let part = dir.join(format!(".{key}.part").replace('/', "_"));
    let mut out = fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut body = resp.body_mut().as_reader();
    let mut buf = vec![0u8; 64 * 1024];
    let (mut got, mut told) = (0u64, Instant::now());
    let result = (|| -> std::io::Result<()> {
        loop {
            let n = body.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            got += n as u64;
            if told.elapsed() > Duration::from_millis(120) {
                told = Instant::now();
                let _ = app.emit("kept", Progress { key: key.into(), got, total });
            }
        }
        out.flush()
    })();
    drop(out);
    if let Err(e) = result.and_then(|_| fs::rename(&part, &file)) {
        let _ = fs::remove_file(&part);
        return Err(e.to_string());
    }
    Ok(file.to_string_lossy().into_owned())
}

// 要らぬ歌　棚の中だけ　片付ける
#[tauri::command]
pub fn forget<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
    let dir = shelf(&app)?;
    let p = Path::new(&path);
    if p.parent() != Some(dir.as_path()) || key_of(&p.file_name().unwrap_or_default().to_string_lossy()).is_none() {
        return Err("outside the shelf".into());
    }
    fs::remove_file(p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 括弧より　駅と歌との　名を拾う
    #[test]
    fn names() {
        assert_eq!(key_of("Mariya - Plastic Love [kksk_ab2c3d].m4a").as_deref(), Some("kksk/ab2c3d"));
        assert_eq!(key_of("x [kksk_ab].txt"), None);
        assert_eq!(key_of("x [KK_ab].mp3"), None);
        assert_eq!(clean("AC/DC: Back?"), "AC_DC_ Back_");
    }
}
