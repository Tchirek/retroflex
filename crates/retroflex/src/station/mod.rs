mod httpread;
mod scan;
mod serve;
mod source;
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

#[derive(Args)]
pub struct StationArgs {
    #[arg(long, env = "RETROFLEX_NAME", help = "Station name (a-z, 0-9, -), used in links")]
    name: String,
    #[arg(long = "library", short = 'l', required = true, help = "Music folder or AList folder URL; repeatable")]
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
}

// 駅ひとつ　棚と目録　鍵を持つ
pub struct Station {
    pub name: String,
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
    let sources = a
        .libraries
        .iter()
        .map(|l| Source::parse(l, a.alist_token.clone()).map(Arc::new))
        .collect::<Result<Vec<_>>>()?;

    tracing::info!("scanning {} source(s)", sources.len());
    let first = scan::scan(&a.name, &sources, &cache).await?;
    tracing::info!("{} tracks, {} covers", first.items.len(), first.covers.len());
    let st = Arc::new(Station { name: a.name.clone(), sources, cache, index: RwLock::new(Arc::new(first)) });

    let (ws, http) = hub_urls(&a.hub);
    println!("\n  {http}/#/s/{}/{key}\n", a.name);

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
