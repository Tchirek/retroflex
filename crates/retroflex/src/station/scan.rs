use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use lofty::config::ParseOptions;
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use super::source::{Entry, Source};
use crate::proto::{Library, Track};
use crate::util::short_hash;

const AUDIO: &[(&str, &str)] = &[
    ("mp3", "audio/mpeg"),
    ("m4a", "audio/mp4"),
    ("m4b", "audio/mp4"),
    ("mp4", "audio/mp4"),
    ("aac", "audio/aac"),
    ("flac", "audio/flac"),
    ("ogg", "audio/ogg"),
    ("oga", "audio/ogg"),
    ("opus", "audio/ogg"),
    ("wav", "audio/wav"),
    ("webm", "audio/webm"),
];
const COVER_NAMES: &[&str] = &["cover", "folder", "front", "album", "albumart"];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png"];
const PARALLEL: usize = 6;

// 在り処のみ　記して中身は　棚の奥
pub enum Asset {
    File { src: usize, rel: String },
    Cached(PathBuf),
    Inline(String),
}

// 一曲の　住まいを示す　小さき札
pub struct Item {
    pub src: usize,
    pub rel: String,
    pub size: u64,
    pub mime: String,
    pub lyrics: Option<Asset>,
}

// 目録の　頁と鍵を　ひと綴じに
pub struct Index {
    pub library: Bytes,
    pub items: HashMap<String, Item>,
    pub covers: HashMap<String, Asset>,
}

// 一度読めば　二度は問わじと　書き残す
#[derive(Clone, Default, Serialize, Deserialize)]
struct Probed {
    size: u64,
    stamp: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    track: Option<u32>,
    disc: Option<u32>,
    year: Option<u16>,
    duration: f64,
    cover: Option<String>,
    lyrics: Option<String>,
}

// 歌か詞か　絵かを名前の　尾で知る
pub fn is_media(rel: &str) -> bool {
    let e = ext(rel);
    AUDIO.iter().any(|(x, _)| *x == e) || IMAGE_EXTS.contains(&e.as_str()) || matches!(e.as_str(), "lrc" | "webp" | "cue")
}

fn ext(rel: &str) -> String {
    rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

fn split(rel: &str) -> (&str, &str) {
    rel.rsplit_once('/').unwrap_or(("", rel))
}

fn stem(name: &str) -> &str {
    name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name)
}

// 番号の　衣を脱げば　歌の名よ
fn title_from_file(name: &str) -> String {
    let s = stem(name);
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-');
    let cut = rest.trim_start_matches([' ', '.', '_', '-']);
    let cleaned = if cut.len() < rest.len() && rest.len() < s.len() { cut } else { s };
    if cleaned.is_empty() { s.to_string() } else { cleaned.to_string() }
}

fn nonempty(s: Option<std::borrow::Cow<'_, str>>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

// 頁めくり　名と長さとを　聞き出しぬ
fn probe(src: &Source, entry: &Entry, want_cover: bool, covers: &Path) -> Result<Probed> {
    let reader = src.open(entry)?;
    let opts = ParseOptions::new().read_cover_art(want_cover);
    let tagged = Probe::new(reader).options(opts).guess_file_type()?.read()?;
    let mut out = Probed {
        size: entry.size,
        stamp: entry.stamp.clone(),
        duration: tagged.properties().duration().as_secs_f64(),
        ..Default::default()
    };
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(out);
    };
    out.title = nonempty(tag.title());
    out.artist = nonempty(tag.artist());
    out.album = nonempty(tag.album());
    out.album_artist = tag.get_string(ItemKey::AlbumArtist).map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    out.track = tag.track();
    out.disc = tag.disk();
    out.year = tag.date().map(|d| d.year);
    out.lyrics = tag
        .get_string(ItemKey::Lyrics)
        .or_else(|| tag.get_string(ItemKey::UnsyncLyrics))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);
    if want_cover {
        let pics = tag.pictures();
        let pic = pics.iter().find(|p| p.pic_type() == PictureType::CoverFront).or(pics.first());
        if let Some(pic) = pic {
            let data = pic.data();
            let suffix = if data.starts_with(b"\x89PNG") { "png" } else { "jpg" };
            let name = format!("{}.{suffix}", short_hash(&[data]));
            let path = covers.join(&name);
            if !path.exists() {
                std::fs::write(&path, data)?;
            }
            out.cover = Some(name);
        }
    }
    Ok(out)
}

// 棚を見て　歌の目録　編み直す
pub async fn scan(name: &str, sources: &[Arc<Source>], cache: &Path) -> Result<Index> {
    let covers_dir = cache.join("covers");
    std::fs::create_dir_all(&covers_dir)?;
    let memo_path = cache.join("probe.json");
    let memo: HashMap<String, Probed> = std::fs::read(&memo_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let memo = Arc::new(memo);
    let gate = Arc::new(Semaphore::new(PARALLEL));

    let mut fresh = HashMap::new();
    let mut tracks = Vec::new();
    let mut items = HashMap::new();
    let mut covers = HashMap::new();

    for (si, src) in sources.iter().enumerate() {
        let label = src.label();
        let entries = {
            let s = src.clone();
            tokio::task::spawn_blocking(move || s.list()).await??
        };
        let mut dirs: HashMap<&str, Vec<&Entry>> = HashMap::new();
        for e in &entries {
            dirs.entry(split(&e.rel).0).or_default().push(e);
        }

        let mut jobs = Vec::new();
        for e in &entries {
            let Some(&(_, mime)) = AUDIO.iter().find(|(x, _)| *x == ext(&e.rel)) else { continue };
            let (dir, file) = split(&e.rel);
            let siblings = &dirs[dir];
            let side_cover = COVER_NAMES.iter().find_map(|c| {
                siblings.iter().find(|s| {
                    let n = split(&s.rel).1;
                    stem(n).eq_ignore_ascii_case(c) && IMAGE_EXTS.contains(&ext(n).as_str())
                })
            });
            let side_lrc = siblings.iter().find(|s| {
                let n = split(&s.rel).1;
                ext(n) == "lrc" && stem(n) == stem(file)
            });
            let id = short_hash(&[label.as_bytes(), e.rel.as_bytes()]);
            let cover_id = side_cover.map(|c| {
                let cid = short_hash(&[label.as_bytes(), c.rel.as_bytes(), c.stamp.as_bytes()]);
                covers.insert(cid.clone(), Asset::File { src: si, rel: c.rel.clone() });
                cid
            });

            let (src, entry, memo, gate, covers_dir) = (src.clone(), (*e).clone(), memo.clone(), gate.clone(), covers_dir.clone());
            let want_cover = side_cover.is_none();
            let lrc = side_lrc.map(|l| l.rel.clone());
            jobs.push(tokio::spawn(async move {
                let cached = memo.get(&id).filter(|p| p.size == entry.size && p.stamp == entry.stamp).cloned();
                let probed = match cached {
                    Some(p) => p,
                    None => {
                        let _permit = gate.acquire_owned().await;
                        let (s, en) = (src.clone(), entry.clone());
                        match tokio::task::spawn_blocking(move || probe(&s, &en, want_cover, &covers_dir)).await {
                            Ok(Ok(p)) => p,
                            Ok(Err(err)) => {
                                tracing::warn!("tags {}: {err}", entry.rel);
                                Probed { size: entry.size, stamp: entry.stamp.clone(), ..Default::default() }
                            }
                            Err(err) => return Err(anyhow::Error::from(err)),
                        }
                    }
                };
                Ok((id, entry, mime, cover_id, lrc, probed))
            }));
        }

        for job in jobs {
            let (id, entry, mime, cover_id, lrc, probed) = job.await??;
            let (dir, file) = split(&entry.rel);
            let (parent, grand) = (split(dir).1, split(split(dir).0).1);
            let cover = cover_id.or_else(|| {
                probed.cover.as_ref().map(|n| {
                    let cid = stem(n).to_string();
                    covers.insert(cid.clone(), Asset::Cached(covers_dir.join(n)));
                    cid
                })
            });
            let lyrics = match (lrc, &probed.lyrics) {
                (Some(rel), _) => Some(Asset::File { src: si, rel }),
                (None, Some(text)) => Some(Asset::Inline(text.clone())),
                _ => None,
            };
            tracks.push(Track {
                id: id.clone(),
                title: probed.title.clone().unwrap_or_else(|| title_from_file(file)),
                artist: probed.artist.clone().unwrap_or_else(|| if grand.is_empty() { parent } else { grand }.to_string()),
                album: probed.album.clone().unwrap_or_else(|| parent.to_string()),
                album_artist: probed.album_artist.clone(),
                track: probed.track,
                disc: probed.disc,
                year: probed.year,
                duration: probed.duration,
                mime: mime.to_string(),
                size: entry.size,
                cover,
                lyrics: lyrics.is_some(),
            });
            items.insert(id.clone(), Item { src: si, rel: entry.rel.clone(), size: entry.size, mime: mime.to_string(), lyrics });
            fresh.insert(id, probed);
        }
    }

    gather_various(&mut tracks, &items);
    tracks.sort_by_cached_key(|t| {
        (
            t.album_artist.as_deref().unwrap_or(&t.artist).to_lowercase(),
            t.album.to_lowercase(),
            t.disc.unwrap_or(1),
            t.track.unwrap_or(u32::MAX),
            t.title.to_lowercase(),
        )
    });

    let tmp = memo_path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(&fresh)?)?;
    std::fs::rename(&tmp, &memo_path)?;

    let blurs = blurs(sources, &covers, cache, &gate).await;
    let library = serde_json::to_vec(&Library { station: name.to_string(), tracks, blurs })?;
    Ok(Index { library: Bytes::from(library), items, covers })
}

// 霞む絵の　小さき影を　先に置く
fn blur_of(bytes: &[u8]) -> Result<String> {
    let img = image::load_from_memory(bytes)?.thumbnail(20, 20).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 60).encode_image(&img)?;
    Ok(format!("data:image/jpeg;base64,{}", data_encoding::BASE64.encode(&out)))
}

// 絵の在り処　棚か懐か　確かめて
fn asset_bytes(sources: &[Arc<Source>], asset: &Asset) -> Result<Vec<u8>> {
    Ok(match asset {
        Asset::File { src, rel } => sources[*src].read_all(rel, 32 * 1024 * 1024)?,
        Asset::Cached(path) => std::fs::read(path)?,
        Asset::Inline(text) => text.clone().into_bytes(),
    })
}

// 盤ごとに　霞をひとつ　覚えおく
async fn blurs(
    sources: &[Arc<Source>],
    covers: &HashMap<String, Asset>,
    cache: &Path,
    gate: &Arc<Semaphore>,
) -> BTreeMap<String, String> {
    let path = cache.join("blurs.json");
    let mut known: BTreeMap<String, String> =
        std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    known.retain(|k, _| covers.contains_key(k));
    let mut jobs = Vec::new();
    for (cid, asset) in covers {
        if known.contains_key(cid) {
            continue;
        }
        let asset = match asset {
            Asset::File { src, rel } => Asset::File { src: *src, rel: rel.clone() },
            Asset::Cached(p) => Asset::Cached(p.clone()),
            Asset::Inline(t) => Asset::Inline(t.clone()),
        };
        let (sources, gate, cid) = (sources.to_vec(), gate.clone(), cid.clone());
        jobs.push(tokio::spawn(async move {
            let _permit = gate.acquire_owned().await;
            let made = tokio::task::spawn_blocking(move || asset_bytes(&sources, &asset).and_then(|b| blur_of(&b))).await;
            (cid, made)
        }));
    }
    for job in jobs {
        match job.await {
            Ok((cid, Ok(Ok(data)))) => {
                known.insert(cid, data);
            }
            Ok((cid, Ok(Err(err)))) => tracing::warn!("blur {cid}: {err:#}"),
            _ => {}
        }
    }
    if let Ok(bytes) = serde_json::to_vec(&known) {
        let _ = std::fs::write(&path, bytes);
    }
    known
}

// 声多き　ひとつの盤は　皆のもの
fn gather_various(tracks: &mut [Track], items: &HashMap<String, Item>) {
    let mut artists: HashMap<(usize, String, String), Vec<String>> = HashMap::new();
    for t in tracks.iter().filter(|t| t.album_artist.is_none()) {
        let it = &items[&t.id];
        let key = (it.src, split(&it.rel).0.to_string(), t.album.clone());
        let v = artists.entry(key).or_default();
        if !v.contains(&t.artist) {
            v.push(t.artist.clone());
        }
    }
    for t in tracks.iter_mut().filter(|t| t.album_artist.is_none()) {
        let it = &items[&t.id];
        let key = (it.src, split(&it.rel).0.to_string(), t.album.clone());
        if artists.get(&key).is_some_and(|v| v.len() > 1) {
            t.album_artist = Some("Various Artists".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 番号を　剥げば素顔の　題が見ゆ
    #[test]
    fn titles_from_names() {
        assert_eq!(title_from_file("09 Plastic Love.m4a"), "Plastic Love");
        assert_eq!(title_from_file("1-13 Hallelujah.m4a"), "Hallelujah");
        assert_eq!(title_from_file("02. Time.flac"), "Time");
        assert_eq!(title_from_file("1999.mp3"), "1999");
        assert_eq!(title_from_file("Breathe.mp3"), "Breathe");
    }
}
