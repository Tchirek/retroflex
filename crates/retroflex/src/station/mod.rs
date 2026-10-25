mod hot;
mod httpread;
mod s3;
mod scan;
mod serve;
mod source;
pub mod sync;
mod uplink;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Args;
use tokio::sync::watch;

use crate::util::{key_hash, token};
use scan::Index;
use source::Source;

// 倉の鍵　四つの札を　ひと綴じに
#[derive(Args, Clone)]
pub struct BucketArgs {
    #[arg(long = "s3-endpoint", env = "RETROFLEX_S3_ENDPOINT", help = "S3 endpoint, e.g. https://<account>.r2.cloudflarestorage.com")]
    endpoint: Option<String>,
    #[arg(long = "s3-region", env = "RETROFLEX_S3_REGION", default_value = "auto", help = "S3 region (R2 uses auto)")]
    region: String,
    #[arg(long = "s3-key", env = "RETROFLEX_S3_KEY", hide_env_values = true, help = "S3 access key ID")]
    key: Option<String>,
    #[arg(long = "s3-secret", env = "RETROFLEX_S3_SECRET", hide_env_values = true, help = "S3 secret access key")]
    secret: Option<String>,
}

impl BucketArgs {
    pub fn client(&self) -> Result<Option<s3::S3>> {
        match (&self.endpoint, &self.key, &self.secret) {
            (Some(e), Some(k), Some(s)) => Ok(Some(s3::S3::new(e, &self.region, k, s)?)),
            _ => Ok(None),
        }
    }
}

#[derive(Args)]
pub struct StationArgs {
    #[arg(long, env = "RETROFLEX_NAME", help = "Station name (a-z, 0-9, -), used in links")]
    name: String,
    #[arg(
        long = "library",
        short = 'l',
        required = true,
        env = "RETROFLEX_LIBRARY",
        value_delimiter = ';',
        help = "Music folder, AList folder URL or r2://bucket/prefix; repeatable"
    )]
    libraries: Vec<String>,
    #[arg(long, env = "RETROFLEX_HUB", help = "Hub URL, e.g. https://listen.example.org")]
    hub: String,
    #[arg(long, env = "RETROFLEX_HUB_TOKEN", hide_env_values = true, help = "Token the hub expects from stations")]
    token: String,
    #[arg(long, env = "RETROFLEX_ALIST_TOKEN", hide_env_values = true, help = "AList token, if the folder is private")]
    alist_token: Option<String>,
    #[arg(long, env = "RETROFLEX_CACHE", help = "Where tags, covers and the station key are kept")]
    cache: Option<PathBuf>,
    #[arg(long, default_value_t = 30, help = "Minutes between library rescans")]
    rescan: u64,
    #[arg(long, env = "RETROFLEX_PUBLIC_URL", help = "Hub URL to print in links, if it differs from --hub")]
    public: Option<String>,
    #[arg(long, env = "RETROFLEX_OPEN", help = "Let anyone on the hub browse this library and open rooms")]
    open: bool,
    #[arg(long, env = "RETROFLEX_RELAY", help = "Never hand out bucket-endpoint links; use the media worker or stream through the station")]
    relay: bool,
    #[arg(long = "media-url", env = "RETROFLEX_MEDIA_URL", help = "Media worker that serves bucket objects, e.g. https://media.example.org")]
    media_url: Option<String>,
    #[arg(long = "media-secret", env = "RETROFLEX_MEDIA_SECRET", hide_env_values = true, help = "Secret shared with the media worker")]
    media_secret: Option<String>,
    #[arg(long = "hot-mb", env = "RETROFLEX_HOT_MB", default_value_t = 2048, help = "Disk kept for recently relayed bucket audio")]
    hot_mb: u64,
    #[command(flatten)]
    bucket: BucketArgs,
}

// 駅ひとつ　棚と目録　鍵を持つ
pub struct Station {
    pub name: String,
    pub open: bool,
    pub direct: bool,
    pub media: Option<(String, String)>,
    pub hot: Option<Arc<hot::Hot>>,
    pub sources: Vec<Arc<Source>>,
    pub cache: PathBuf,
    index: RwLock<Arc<Index>>,
}

impl Station {
    pub fn index(&self) -> Arc<Index> {
        self.index.read().unwrap().clone()
    }

    fn set_index(&self, index: Index) {
        *self.index.write().unwrap() = Arc::new(index);
    }
}

pub fn valid_name(name: &str) -> bool {
    (1..=32).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

// 道の名を　声の道へと　言い換えて
fn hub_urls(hub: &str) -> (String, String) {
    let hub = hub.trim_end_matches('/');
    let rest = hub.split_once("://").map(|(_, r)| r).unwrap_or(hub);
    let secure = !(hub.starts_with("http://") || hub.starts_with("ws://"));
    if secure {
        (format!("wss://{rest}"), format!("https://{rest}"))
    } else {
        (format!("ws://{rest}"), format!("http://{rest}"))
    }
}

// 合鍵を　一度作れば　永く持つ
fn station_key(cache: &std::path::Path) -> Result<String> {
    let path = cache.join("station.key");
    if let Ok(k) = std::fs::read_to_string(&path) {
        let k = k.trim().to_string();
        if !k.is_empty() {
            return Ok(k);
        }
    }
    let k = token(20);
    std::fs::write(&path, &k)?;
    Ok(k)
}

// 灯を入れ　棚を数えて　網に出る
pub async fn run(a: StationArgs) -> Result<()> {
    if !valid_name(&a.name) {
        bail!("station name must be 1-32 chars of a-z, 0-9 or -");
    }
    let cache = a
        .cache
        .clone()
        .unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("retroflex").join(&a.name));
    std::fs::create_dir_all(&cache).with_context(|| format!("cache {}", cache.display()))?;
    let key = station_key(&cache)?;
    let bucket = a.bucket.client()?;
    let sources = a
        .libraries
        .iter()
        .map(|l| Source::parse(l, a.alist_token.clone(), bucket.as_ref()).map(Arc::new))
        .collect::<Result<Vec<_>>>()?;

    let media = match (&a.media_url, &a.media_secret) {
        (Some(u), Some(s)) if !u.is_empty() && !s.is_empty() => Some((u.trim_end_matches('/').to_string(), s.clone())),
        _ => None,
    };
    let hot = if sources.iter().any(|s| s.is_bucket()) && a.hot_mb > 0 {
        Some(hot::Hot::open(cache.join("hot"), a.hot_mb * 1024 * 1024)?)
    } else {
        None
    };
    tracing::info!("scanning {} source(s)", sources.len());
    let first = scan::scan(&a.name, &sources, &cache).await?;
    tracing::info!("{} tracks, {} covers", first.items.len(), first.covers.len());
    let st = Arc::new(Station { name: a.name.clone(), open: a.open, direct: !a.relay, media, hot, sources, cache, index: RwLock::new(Arc::new(first)) });

    let (ws, http) = hub_urls(&a.hub);
    let shown = a.public.as_deref().map(|p| p.trim_end_matches('/').to_string()).unwrap_or(http);
    println!("\n  {shown}/#/s/{}/{key}\n", a.name);

    let (changed_tx, changed_rx) = watch::channel(0u64);
    let rescanner = st.clone();
    let every = Duration::from_secs(a.rescan.max(1) * 60);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(every).await;
            match scan::scan(&rescanner.name, &rescanner.sources, &rescanner.cache).await {
                Ok(next) if next.library != rescanner.index().library => {
                    tracing::info!("library changed: {} tracks", next.items.len());
                    rescanner.set_index(next);
                    changed_tx.send_modify(|v| *v += 1);
                }
                Ok(_) => {}
                Err(err) => tracing::warn!("rescan: {err:#}"),
            }
        }
    });

    uplink::run(st, ws, a.token, key_hash(&key), changed_rx).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 名の形　正しきものを　通しけり
    #[test]
    fn names_and_urls() {
        assert!(valid_name("kksk"));
        assert!(!valid_name("Kksk"));
        assert!(!valid_name(""));
        assert_eq!(hub_urls("https://a.b/").0, "wss://a.b");
        assert_eq!(hub_urls("ws://127.0.0.1:7746").1, "http://127.0.0.1:7746");
        assert_eq!(hub_urls("a.b").0, "wss://a.b");
    }
}
