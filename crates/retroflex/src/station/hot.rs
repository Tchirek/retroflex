use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use super::source::Source;

// よく聴く　歌だけ手元に　置いておく
pub struct Hot {
    dir: PathBuf,
    cap: u64,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    files: HashMap<String, (u64, u64)>,
    total: u64,
    busy: HashSet<String>,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}

impl Hot {
    // 棚を見て　前に置いたる　歌を知る
    pub fn open(dir: PathBuf, cap: u64) -> Result<Arc<Self>> {
        std::fs::create_dir_all(&dir)?;
        let mut inner = Inner::default();
        for e in std::fs::read_dir(&dir)?.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".part") {
                let _ = std::fs::remove_file(&path);
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            let used = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            inner.total += meta.len();
            inner.files.insert(name, (meta.len(), used));
        }
        let hot = Arc::new(Self { dir, cap, inner: Mutex::new(inner) });
        hot.evict();
        Ok(hot)
    }

    // 手元にあれば　その一節を　すぐ渡す
    pub fn read(&self, id: &str, start: u64, len: u64) -> Option<Box<dyn Read + Send>> {
        {
            let mut inner = self.inner.lock().unwrap();
            let entry = inner.files.get_mut(id)?;
            entry.1 = now();
        }
        let path = self.dir.join(id);
        let mut f = File::open(&path).ok()?;
        let _ = f.set_modified(SystemTime::now());
        f.seek(SeekFrom::Start(start)).ok()?;
        Some(Box::new(f.take(len)))
    }

    // 初めての　歌は裏手で　写し取る
    pub fn fill(self: &Arc<Self>, id: &str, src: Arc<Source>, rel: String, size: u64) {
        if size == 0 || size > self.cap / 4 {
            return;
        }
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.files.contains_key(id) || !inner.busy.insert(id.to_string()) {
                return;
            }
        }
        let (hot, id) = (self.clone(), id.to_string());
        std::thread::spawn(move || {
            let part = hot.dir.join(format!("{id}.part"));
            let copied = (|| -> Result<u64> {
                let mut body = src.read_range(&rel, 0, size)?;
                let mut out = File::create(&part)?;
                Ok(std::io::copy(&mut body, &mut out)?)
            })();
            let mut inner = hot.inner.lock().unwrap();
            inner.busy.remove(&id);
            match copied {
                Ok(n) if n == size && std::fs::rename(&part, hot.dir.join(&id)).is_ok() => {
                    inner.total += n;
                    inner.files.insert(id, (n, now()));
                    drop(inner);
                    hot.evict();
                }
                other => {
                    let _ = std::fs::remove_file(&part);
                    if let Err(err) = other {
                        tracing::warn!("hot cache {rel}: {err:#}");
                    }
                }
            }
        });
    }

    // 棚溢れ　久しく聴かぬ　歌を出す
    fn evict(&self) {
        let mut inner = self.inner.lock().unwrap();
        while inner.total > self.cap {
            let Some(oldest) = inner.files.iter().min_by_key(|(_, (_, used))| *used).map(|(k, _)| k.clone()) else { break };
            if let Some((size, _)) = inner.files.remove(&oldest) {
                inner.total = inner.total.saturating_sub(size);
                let _ = std::fs::remove_file(self.dir.join(&oldest));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 古き順　溢れし分を　手放しぬ
    #[test]
    fn evicts_least_recent() {
        let dir = std::env::temp_dir().join(format!("retroflex-hot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (name, used) in [("a", 1), ("b", 2), ("c", 3)] {
            let p = dir.join(name);
            std::fs::write(&p, vec![0u8; 400]).unwrap();
            File::options().write(true).open(&p).unwrap().set_modified(UNIX_EPOCH + std::time::Duration::from_secs(used)).unwrap();
        }
        let hot = Hot::open(dir.clone(), 900).unwrap();
        assert!(!dir.join("a").exists());
        assert!(hot.read("b", 0, 10).is_some());
        assert!(hot.read("a", 0, 10).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
