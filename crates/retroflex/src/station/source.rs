use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use serde::Deserialize;

use super::httpread::HttpReader;

const SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'.').remove(b'~');

// 棚の上　名と重さと　日付あり
#[derive(Debug, Clone)]
pub struct Entry {
    pub rel: String,
    pub size: u64,
    pub stamp: String,
}

pub trait ReadSeek: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeek for T {}

// 手元の棚　あるいは遠き　網の倉
pub enum Source {
    Local { root: PathBuf },
    Alist { base: String, root: String, token: Option<String>, agent: ureq::Agent, signs: Mutex<HashMap<String, String>> },
}

#[derive(Deserialize)]
struct AlistResp<T> {
    code: u16,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Deserialize)]
struct AlistList {
    content: Option<Vec<AlistItem>>,
}

#[derive(Deserialize)]
struct AlistItem {
    name: String,
    size: u64,
    is_dir: bool,
    #[serde(default)]
    modified: String,
    #[serde(default)]
    sign: String,
}

impl Source {
    // 道しるべ　網か手元か　見分けおく
    pub fn parse(spec: &str, token: Option<String>) -> Result<Self> {
        if spec.starts_with("http://") || spec.starts_with("https://") {
            let scheme_end = spec.find("://").unwrap() + 3;
            let path_start = spec[scheme_end..].find('/').map(|i| i + scheme_end).unwrap_or(spec.len());
            let base = spec[..path_start].to_string();
            let root = percent_decode_str(&spec[path_start..]).decode_utf8()?.trim_end_matches('/').to_string();
            let agent = ureq::Agent::config_builder()
                .timeout_connect(Some(std::time::Duration::from_secs(10)))
                .timeout_recv_response(Some(std::time::Duration::from_secs(30)))
                .build()
                .new_agent();
            Ok(Source::Alist { base, root, token, agent, signs: Mutex::new(HashMap::new()) })
        } else {
            let root = std::fs::canonicalize(spec).with_context(|| format!("library {spec}"))?;
            if !root.is_dir() {
                bail!("{spec} is not a directory");
            }
            Ok(Source::Local { root })
        }
    }

    // 生まれ名を　変えずに呼べば　縁続く
    pub fn label(&self) -> String {
        match self {
            Source::Local { root } => root.to_string_lossy().replace('\\', "/"),
            Source::Alist { base, root, .. } => format!("{base}{root}"),
        }
    }

    // 隅々を　ひとつ残らず　歩みけり
    pub fn list(&self) -> Result<Vec<Entry>> {
        match self {
            Source::Local { root } => {
                let mut out = Vec::new();
                for e in walkdir::WalkDir::new(root).follow_links(true) {
                    let e = match e {
                        Ok(e) => e,
                        Err(err) => {
                            tracing::warn!("skip: {err}");
                            continue;
                        }
                    };
                    if !e.file_type().is_file() {
                        continue;
                    }
                    let meta = e.metadata()?;
                    let rel = e.path().strip_prefix(root)?.to_string_lossy().replace('\\', "/");
                    let stamp = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_secs().to_string())
                        .unwrap_or_default();
                    out.push(Entry { rel, size: meta.len(), stamp });
                }
                Ok(out)
            }
            Source::Alist { signs, .. } => {
                let mut out = Vec::new();
                let mut fresh = HashMap::new();
                self.alist_walk("", &mut out, &mut fresh)?;
                *signs.lock().unwrap() = fresh;
                Ok(out)
            }
        }
    }

    // 階を降り　また階を降り　底の歌
    fn alist_walk(&self, rel: &str, out: &mut Vec<Entry>, signs: &mut HashMap<String, String>) -> Result<()> {
        let Source::Alist { base, root, token, agent, .. } = self else { unreachable!() };
        let path = if rel.is_empty() { root.clone() } else { format!("{root}/{rel}") };
        let mut req = agent.post(format!("{base}/api/fs/list"));
        if let Some(t) = token {
            req = req.header("Authorization", t);
        }
        let body = serde_json::json!({ "path": path, "password": "", "page": 1, "per_page": 0, "refresh": false });
        let resp: AlistResp<AlistList> = req.send_json(body)?.body_mut().read_json()?;
        if resp.code != 200 {
            bail!("alist {path}: {} {}", resp.code, resp.message);
        }
        for item in resp.data.and_then(|d| d.content).unwrap_or_default() {
            let child = if rel.is_empty() { item.name.clone() } else { format!("{rel}/{}", item.name) };
            if item.is_dir {
                self.alist_walk(&child, out, signs)?;
            } else {
                if !item.sign.is_empty() {
                    signs.insert(child.clone(), item.sign);
                }
                out.push(Entry { rel: child, size: item.size, stamp: item.modified });
            }
        }
        Ok(())
    }

    // 網の上　ひとつの頁に　宛を書く
    fn alist_url(&self, rel: &str) -> String {
        let Source::Alist { base, root, signs, .. } = self else { unreachable!() };
        let full = format!("{root}/{rel}");
        let encoded: Vec<String> = full.split('/').map(|s| utf8_percent_encode(s, SEGMENT).to_string()).collect();
        let mut url = format!("{base}/d{}", encoded.join("/"));
        if let Some(sign) = signs.lock().unwrap().get(rel) {
            url.push_str("?sign=");
            url.push_str(sign);
        }
        url
    }

    // 頁繰り　行きつ戻りつ　読める本
    pub fn open(&self, entry: &Entry) -> Result<Box<dyn ReadSeek>> {
        match self {
            Source::Local { root } => Ok(Box::new(BufReader::new(File::open(root.join(&entry.rel))?))),
            Source::Alist { agent, .. } => Ok(Box::new(HttpReader::new(agent.clone(), self.alist_url(&entry.rel), entry.size))),
        }
    }

    // 切り取りて　その一節を　流しけり
    pub fn read_range(&self, rel: &str, start: u64, len: u64) -> Result<Box<dyn Read + Send>> {
        match self {
            Source::Local { root } => {
                let mut f = File::open(root.join(rel))?;
                f.seek(SeekFrom::Start(start))?;
                Ok(Box::new(f.take(len)))
            }
            Source::Alist { agent, .. } => {
                let end = start + len.max(1) - 1;
                let resp = agent
                    .get(self.alist_url(rel))
                    .header("Range", format!("bytes={start}-{end}"))
                    .call()?;
                Ok(Box::new(resp.into_body().into_reader().take(len)))
            }
        }
    }

    // 小さき紙　まるごと読みて　手に載せる
    pub fn read_all(&self, rel: &str, limit: u64) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.read_range(rel, 0, limit)?.read_to_end(&mut out)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 網の名を　分けて解けば　根が見える
    #[test]
    fn parses_alist_urls() {
        let s = Source::parse("http://10.0.0.2:5244/S:%20drive/kksk/", None).unwrap();
        assert_eq!(s.label(), "http://10.0.0.2:5244/S: drive/kksk");
        assert_eq!(s.alist_url("A b/01 x.m4a"), "http://10.0.0.2:5244/d/S%3A%20drive/kksk/A%20b/01%20x.m4a");
    }
}
