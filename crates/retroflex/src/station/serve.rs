use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::Result;

use super::Station;
use super::scan::Asset;
use crate::proto::Op;

const THUMB: u32 = 512;
const SMALL_LIMIT: u64 = 32 * 1024 * 1024;

// 返し文　頭と身とを　揃えたり
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Box<dyn Read + Send>,
}

impl Reply {
    fn empty(status: u16) -> Self {
        Self { status, headers: Vec::new(), body: Box::new(std::io::empty()) }
    }

    // 倉を指し　ここより直に　汲めと言う
    fn redirect(url: String) -> Self {
        let mut r = Self::empty(302);
        r.headers.push(("location".into(), url));
        r.headers.push(("cache-control".into(), "no-store".into()));
        r
    }

    fn bytes(ctype: &str, data: Vec<u8>) -> Self {
        let headers = vec![
            ("content-type".into(), ctype.into()),
            ("content-length".into(), data.len().to_string()),
        ];
        Self { status: 200, headers, body: Box::new(Cursor::new(data)) }
    }
}

// 求めには　ひとつ残らず　応えけり
pub fn respond(st: &Station, op: &Op, range: Option<&str>) -> Reply {
    match answer(st, op, range) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!("{op:?}: {err:#}");
            Reply::empty(502)
        }
    }
}

// 端と端　頼まれし幅を　測りおく
pub fn parse_range(header: &str, size: u64) -> Option<Result<(u64, u64), ()>> {
    let spec = header.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let (a, b) = (a.trim(), b.trim());
    let span = if a.is_empty() {
        let n: u64 = b.parse().ok()?;
        if n == 0 {
            return Some(Err(()));
        }
        (size.saturating_sub(n), size.checked_sub(1)?)
    } else {
        let start: u64 = a.parse().ok()?;
        let end = if b.is_empty() { size.saturating_sub(1) } else { b.parse::<u64>().ok()?.min(size.saturating_sub(1)) };
        (start, end)
    };
    if span.0 >= size || span.0 > span.1 { Some(Err(())) } else { Some(Ok(span)) }
}

fn answer(st: &Station, op: &Op, range: Option<&str>) -> Result<Reply> {
    let index = st.index();
    match op {
        Op::Library => Ok(Reply::bytes("application/json", index.library.to_vec())),
        Op::Audio { track } => {
            let Some(item) = index.items.get(track) else { return Ok(Reply::empty(404)) };
            if st.direct {
                if let Some(url) = st.sources[item.src].direct_url(&item.rel) {
                    return Ok(Reply::redirect(url));
                }
            }
            let size = item.size;
            let (status, start, end) = match range.and_then(|r| parse_range(r, size)) {
                Some(Ok((a, b))) => (206, a, b),
                Some(Err(())) => {
                    let mut r = Reply::empty(416);
                    r.headers.push(("content-range".into(), format!("bytes */{size}")));
                    return Ok(r);
                }
                None if size == 0 => return Ok(Reply::bytes(&item.mime, Vec::new())),
                None => (200, 0, size - 1),
            };
            let len = end - start + 1;
            let body = st.sources[item.src].read_range(&item.rel, start, len)?;
            let mut headers = vec![
                ("content-type".to_string(), item.mime.clone()),
                ("content-length".to_string(), len.to_string()),
                ("accept-ranges".to_string(), "bytes".to_string()),
            ];
            if status == 206 {
                headers.push(("content-range".into(), format!("bytes {start}-{end}/{size}")));
            }
            Ok(Reply { status, headers, body })
        }
        Op::Cover { cover, thumb } => {
            let Some(asset) = index.covers.get(cover) else { return Ok(Reply::empty(404)) };
            if let (false, true, Asset::File { src, rel }) = (*thumb, st.direct, asset) {
                if let Some(url) = st.sources[*src].direct_url(rel) {
                    return Ok(Reply::redirect(url));
                }
            }
            let data = if *thumb { thumbnail(st, cover, asset)? } else { load(st, asset)? };
            let ctype = if data.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
            Ok(Reply::bytes(ctype, data))
        }
        Op::Lyrics { track } => {
            let Some(asset) = index.items.get(track).and_then(|i| i.lyrics.as_ref()) else {
                return Ok(Reply::empty(404));
            };
            let text = decode_text(&load(st, asset)?);
            Ok(Reply::bytes("text/plain; charset=utf-8", text.into_bytes()))
        }
    }
}

// 棚の奥　あるいは懐　取り出して
fn load(st: &Station, asset: &Asset) -> Result<Vec<u8>> {
    Ok(match asset {
        Asset::File { src, rel } => st.sources[*src].read_all(rel, SMALL_LIMIT)?,
        Asset::Cached(path) => std::fs::read(path)?,
        Asset::Inline(text) => text.clone().into_bytes(),
    })
}

// 大き絵を　掌ほどに　縮めけり
fn thumbnail(st: &Station, cover: &str, asset: &Asset) -> Result<Vec<u8>> {
    let path = st.cache.join("thumbs").join(format!("{cover}.jpg"));
    if let Ok(data) = std::fs::read(&path) {
        return Ok(data);
    }
    let img = image::load_from_memory(&load(st, asset)?)?.thumbnail(THUMB, THUMB).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 84).encode_image(&img)?;
    write_atomic(&path, &out)?;
    Ok(out)
}

fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

// 異国の字　読み解きてなお　歌は歌
pub fn decode_text(bytes: &[u8]) -> String {
    if let Some((enc, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        return enc.decode_without_bom_handling(&bytes[bom..]).0.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::GB18030.decode(bytes).0.into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 幅を問い　答え違わぬ　物差しよ
    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-", 100), Some(Ok((0, 99))));
        assert_eq!(parse_range("bytes=10-19", 100), Some(Ok((10, 19))));
        assert_eq!(parse_range("bytes=90-200", 100), Some(Ok((90, 99))));
        assert_eq!(parse_range("bytes=-10", 100), Some(Ok((90, 99))));
        assert_eq!(parse_range("bytes=100-", 100), Some(Err(())));
        assert_eq!(parse_range("bytes=0-1,5-6", 100), None);
        assert_eq!(parse_range("items=0-1", 100), None);
    }

    // 古き字も　新しき字も　ひとつ声
    #[test]
    fn decodes_legacy_lyrics() {
        let (gbk, _, _) = encoding_rs::GB18030.encode("[00:01.00]晚风");
        assert_eq!(decode_text(&gbk), "[00:01.00]晚风");
        assert_eq!(decode_text("\u{feff}[ti:x]".as_bytes()), "[ti:x]");
    }
}
