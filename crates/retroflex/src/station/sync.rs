use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use anyhow::{Result, bail};
use clap::Args;
use tokio::sync::Semaphore;

use super::BucketArgs;
use super::scan::is_media;
use super::source::Source;

#[derive(Args)]
pub struct SyncArgs {
    #[arg(long, help = "Folder or AList folder URL to read from")]
    from: String,
    #[arg(long, help = "Bucket to write to, e.g. r2://retroflex/kksk")]
    to: String,
    #[arg(long, env = "RETROFLEX_ALIST_TOKEN", hide_env_values = true, help = "AList token, if the source is private")]
    alist_token: Option<String>,
    #[arg(long, help = "Copy every file, not only audio, lyrics and pictures")]
    all: bool,
    #[arg(long, help = "Show what would be copied without copying")]
    dry_run: bool,
    #[arg(long, default_value_t = 4, help = "Uploads in flight at once")]
    parallel: usize,
    #[command(flatten)]
    bucket: BucketArgs,
}

// 棚の歌　倉へ写して　欠けを埋む
pub async fn run(a: SyncArgs) -> Result<()> {
    let client = a.bucket.client()?;
    let from = Arc::new(Source::parse(&a.from, a.alist_token.clone(), client.as_ref())?);
    let to = Arc::new(Source::parse(&a.to, None, client.as_ref())?);
    if to.direct_url("probe").is_none() {
        bail!("--to must be a bucket (r2://bucket/prefix)");
    }
    let (src, dst) = (from.clone(), to.clone());
    let (local, remote) = tokio::task::spawn_blocking(move || Ok::<_, anyhow::Error>((src.list()?, dst.list()?))).await??;
    let have: HashMap<String, u64> = remote.into_iter().map(|e| (e.rel, e.size)).collect();
    let todo: Vec<_> = local
        .into_iter()
        .filter(|e| a.all || is_media(&e.rel))
        .filter(|e| have.get(&e.rel) != Some(&e.size))
        .collect();
    let total: u64 = todo.iter().map(|e| e.size).sum();
    println!("{} files to copy, {:.1} MB", todo.len(), total as f64 / 1e6);
    if a.dry_run {
        for e in &todo {
            println!("  {}", e.rel);
        }
        return Ok(());
    }

    let gate = Arc::new(Semaphore::new(a.parallel.max(1)));
    let sent = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let mut jobs = Vec::new();
    for e in todo {
        let (from, to, gate, sent, failed) = (from.clone(), to.clone(), gate.clone(), sent.clone(), failed.clone());
        jobs.push(tokio::spawn(async move {
            let _permit = gate.acquire_owned().await;
            let rel = e.rel.clone();
            let done = tokio::task::spawn_blocking(move || {
                let body = from.read_range(&e.rel, 0, e.size)?;
                to.put(&e.rel, e.size, body)
            })
            .await;
            match done {
                Ok(Ok(())) => {
                    let now = sent.fetch_add(e.size, Ordering::Relaxed) + e.size;
                    println!("{:5.1}%  {rel}", now as f64 * 100.0 / total.max(1) as f64);
                }
                Ok(Err(err)) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("failed {rel}: {err:#}");
                }
                Err(err) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("failed {rel}: {err}");
                }
            }
        }));
    }
    for j in jobs {
        let _ = j.await;
    }
    let failed = failed.load(Ordering::Relaxed);
    if failed > 0 {
        bail!("{failed} file(s) failed; run again to retry them");
    }
    println!("done");
    Ok(())
}
