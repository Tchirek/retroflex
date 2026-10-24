use std::collections::HashMap;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, bail};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::task::AbortHandle;
use tokio_tungstenite::tungstenite::Message;

use super::{Station, serve};
use crate::proto::{CHUNK, Down, PROTOCOL, Up, WINDOW, frame};

// 流れごと　受け取る許しの　升ひとつ
struct Flow {
    credit: Arc<Semaphore>,
    task: Option<AbortHandle>,
}

type Flows = Arc<Mutex<HashMap<u32, Flow>>>;

fn text(up: &Up) -> Message {
    Message::Text(serde_json::to_string(up).expect("serializable").into())
}

// 切れてなお　また結びゆく　糸電話
pub async fn run(st: Arc<Station>, hub: String, token: String, key: String, mut changed: watch::Receiver<u64>) {
    let mut wait = 1;
    loop {
        match session(&st, &hub, &token, &key, &mut changed).await {
            Ok(()) => {
                tracing::info!("uplink closed");
                wait = 1;
            }
            Err(err) => tracing::warn!("uplink: {err:#}"),
        }
        tokio::time::sleep(Duration::from_secs(wait)).await;
        wait = (wait * 2).min(30);
    }
}

async fn session(st: &Arc<Station>, hub: &str, token: &str, key: &str, changed: &mut watch::Receiver<u64>) -> Result<()> {
    let url = format!("{}/api/uplink", hub.trim_end_matches('/'));
    let (ws, _) = tokio_tungstenite::connect_async(url.as_str()).await?;
    let (mut sink, mut stream) = ws.split();
    let hello = Up::Hello { name: st.name.clone(), token: token.into(), key: key.into(), version: PROTOCOL };
    sink.send(text(&hello)).await?;

    // 名を告げて　返る言葉を　待つ一拍
    match tokio::time::timeout(Duration::from_secs(15), stream.next()).await? {
        Some(Ok(Message::Text(t))) => match serde_json::from_str::<Down>(&t)? {
            Down::Welcome => {}
            Down::Reject { reason } => bail!("rejected: {reason}"),
            other => bail!("unexpected {other:?}"),
        },
        other => bail!("closed before welcome: {other:?}"),
    }
    tracing::info!("linked to {hub}");

    let (tx, mut rx) = mpsc::channel::<Message>(64);
    let writer = tokio::spawn(async move {
        while let Some(m) = rx.recv().await {
            if sink.send(m).await.is_err() {
                break;
            }
        }
    });
    let flows: Flows = Arc::default();
    let mut ping = tokio::time::interval(Duration::from_secs(20));

    let result = loop {
        tokio::select! {
            msg = stream.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => break Err(e.into()),
                    None => break Ok(()),
                };
                match msg {
                    Message::Text(t) => match serde_json::from_str::<Down>(&t) {
                        Ok(Down::Req { id, op, range }) => start_flow(st, &tx, &flows, id, op, range),
                        Ok(Down::Ack { id, n }) => {
                            if let Some(f) = flows.lock().unwrap().get(&id) {
                                f.credit.add_permits(n as usize);
                            }
                        }
                        Ok(Down::Cancel { id }) => {
                            if let Some(f) = flows.lock().unwrap().remove(&id) {
                                if let Some(t) = f.task { t.abort() }
                            }
                        }
                        Ok(_) => {}
                        Err(err) => tracing::warn!("bad frame: {err}"),
                    },
                    Message::Close(_) => break Ok(()),
                    _ => {}
                }
            }
            _ = ping.tick() => {
                if tx.send(Message::Ping(Default::default())).await.is_err() { break Ok(()) }
            }
            Ok(()) = changed.changed() => {
                let _ = tx.send(text(&Up::Changed)).await;
            }
        }
    };

    for (_, f) in flows.lock().unwrap().drain() {
        if let Some(t) = f.task { t.abort() }
    }
    writer.abort();
    result
}

// 頼まれて　ひと筋の水　送り出す
fn start_flow(st: &Arc<Station>, tx: &mpsc::Sender<Message>, flows: &Flows, id: u32, op: crate::proto::Op, range: Option<String>) {
    let credit = Arc::new(Semaphore::new(WINDOW));
    flows.lock().unwrap().insert(id, Flow { credit: credit.clone(), task: None });
    let (st, tx, done) = (st.clone(), tx.clone(), flows.clone());
    let task = tokio::spawn(async move {
        let reply = tokio::task::spawn_blocking(move || serve::respond(&st, &op, range.as_deref())).await;
        let reply = reply.unwrap_or_else(|_| serve::Reply { status: 500, headers: Vec::new(), body: Box::new(io::empty()) });
        let head = Up::Head { id, status: reply.status, headers: reply.headers };
        if tx.send(text(&head)).await.is_ok() {
            pump(id, reply.body, &credit, &tx).await;
            let _ = tx.send(text(&Up::End { id })).await;
        }
        done.lock().unwrap().remove(&id);
    });
    if let Some(f) = flows.lock().unwrap().get_mut(&id) {
        f.task = Some(task.abort_handle());
    }
}

// 汲みあげて　許しの分だけ　注ぎけり
async fn pump(id: u32, mut body: Box<dyn Read + Send>, credit: &Semaphore, tx: &mpsc::Sender<Message>) {
    let (ctx, mut crx) = mpsc::channel::<io::Result<Vec<u8>>>(4);
    tokio::task::spawn_blocking(move || {
        loop {
            let mut buf = vec![0u8; CHUNK];
            let mut filled = 0;
            let res = loop {
                match body.read(&mut buf[filled..]) {
                    Ok(0) => break Ok(filled),
                    Ok(n) => {
                        filled += n;
                        if filled == CHUNK {
                            break Ok(filled);
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => break Err(e),
                }
            };
            match res {
                Ok(0) => return,
                Ok(n) => {
                    buf.truncate(n);
                    if ctx.blocking_send(Ok(buf)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = ctx.blocking_send(Err(e));
                    return;
                }
            }
        }
    });
    while let Some(chunk) = crx.recv().await {
        let Ok(chunk) = chunk else { return };
        match credit.acquire_many(chunk.len() as u32).await {
            Ok(p) => p.forget(),
            Err(_) => return,
        }
        if tx.send(Message::Binary(frame(id, &chunk).into())).await.is_err() {
            return;
        }
    }
}
