use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::Response;
use bytes::Bytes;
use futures_util::{SinkExt, Stream, StreamExt};
use tokio::sync::{mpsc, oneshot};

use super::Hub;
use super::rooms::Snap;
use crate::proto::{Down, Library, Op, PROTOCOL, Up, unframe};
use crate::station::valid_name;
use crate::util::same;

// 返し文の　頭ばかりを　先に受く
pub struct Head {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}

struct Pending {
    head: Option<oneshot::Sender<Head>>,
    body: mpsc::Sender<Bytes>,
}

// 遠き駅　一本の糸　繋ぎおく
pub struct Link {
    pub key: String,
    pub open: bool,
    tx: mpsc::UnboundedSender<Message>,
    pending: Mutex<HashMap<u32, Pending>>,
    next: AtomicU32,
    library: Mutex<Option<Bytes>>,
    pool: Mutex<Option<Arc<Vec<Snap>>>>,
    warming: AtomicBool,
}

const LIBRARY_LIMIT: usize = 64 * 1024 * 1024;

fn text(down: &Down) -> Message {
    Message::Text(serde_json::to_string(down).expect("serializable").into())
}

impl Link {
    // 目録は　一度借りれば　手元に置く
    pub async fn catalog(self: &Arc<Self>) -> Result<Bytes, StatusCode> {
        if let Some(b) = self.library.lock().unwrap().clone() {
            return Ok(b);
        }
        let (head, body) = self.request(Op::Library, None).await?;
        if head.status != 200 {
            return Err(StatusCode::BAD_GATEWAY);
        }
        let bytes = body.collect(LIBRARY_LIMIT).await.ok_or(StatusCode::BAD_GATEWAY)?;
        let pool = serde_json::from_slice::<Library>(&bytes)
            .ok()
            .map(|l| Arc::new(l.tracks.iter().map(Snap::from).collect::<Vec<_>>()));
        *self.library.lock().unwrap() = Some(bytes.clone());
        *self.pool.lock().unwrap() = pool;
        Ok(bytes)
    }

    // 籤を引く　歌の名簿が　あるならば
    pub fn pool(&self) -> Option<Arc<Vec<Snap>>> {
        self.pool.lock().unwrap().clone()
    }

    // 名簿なく　そっと使いを　走らせる
    pub fn warm(self: &Arc<Self>) {
        if self.warming.swap(true, Ordering::AcqRel) {
            return;
        }
        let link = self.clone();
        tokio::spawn(async move {
            let _ = link.catalog().await;
            link.warming.store(false, Ordering::Release);
        });
    }

    fn forget(&self) {
        *self.library.lock().unwrap() = None;
        *self.pool.lock().unwrap() = None;
    }

    // 頼み書き　番号振りて　糸に乗す
    pub async fn request(self: &Arc<Self>, op: Op, range: Option<String>) -> Result<(Head, Body), StatusCode> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = mpsc::channel(64);
        self.pending.lock().unwrap().insert(id, Pending { head: Some(head_tx), body: body_tx });
        let body = Body { link: self.clone(), id, rx: body_rx, done: false };
        self.tx.send(text(&Down::Req { id, op, range })).map_err(|_| StatusCode::BAD_GATEWAY)?;
        match tokio::time::timeout(Duration::from_secs(30), head_rx).await {
            Ok(Ok(head)) => Ok((head, body)),
            Ok(Err(_)) => Err(StatusCode::BAD_GATEWAY),
            Err(_) => Err(StatusCode::GATEWAY_TIMEOUT),
        }
    }

    fn on_head(&self, id: u32, head: Head) {
        if let Some(p) = self.pending.lock().unwrap().get_mut(&id) {
            if let Some(h) = p.head.take() {
                let _ = h.send(head);
            }
        }
    }

    // 溢れなば　その流れのみ　堰き止めて
    fn on_chunk(&self, id: u32, data: Bytes) {
        let mut pending = self.pending.lock().unwrap();
        let overflow = pending.get(&id).is_some_and(|p| p.body.try_send(data).is_err());
        if overflow {
            pending.remove(&id);
            let _ = self.tx.send(text(&Down::Cancel { id }));
        }
    }
}

// 汲むたびに　礼をひとこと　送り返す
pub struct Body {
    link: Arc<Link>,
    id: u32,
    rx: mpsc::Receiver<Bytes>,
    done: bool,
}

impl Stream for Body {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => {
                let _ = self.link.tx.send(text(&Down::Ack { id: self.id, n: chunk.len() as u32 }));
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for Body {
    // 聞き手去り　水を止めよと　告げにゆく
    fn drop(&mut self) {
        if !self.done {
            self.link.pending.lock().unwrap().remove(&self.id);
            let _ = self.link.tx.send(text(&Down::Cancel { id: self.id }));
        }
    }
}

impl Body {
    // 一息に　汲み尽くしては　器満つ
    pub async fn collect(mut self, limit: usize) -> Option<Bytes> {
        let mut out = Vec::new();
        while let Some(chunk) = self.next().await {
            out.extend_from_slice(&chunk.ok()?);
            if out.len() > limit {
                return None;
            }
        }
        Some(out.into())
    }
}

// 駅の名を　帳に記して　糸を引く
#[derive(Clone, Default)]
pub struct Stations(Arc<Mutex<HashMap<String, Arc<Link>>>>);

impl Stations {
    pub fn get(&self, name: &str) -> Option<Arc<Link>> {
        self.0.lock().unwrap().get(name).cloned()
    }

    // 誰にでも　開かれし駅の　名を挙げる
    pub fn open(&self) -> Vec<String> {
        let mut names: Vec<String> = self.0.lock().unwrap().iter().filter(|(_, l)| l.open).map(|(n, _)| n.clone()).collect();
        names.sort();
        names
    }
}

pub async fn uplink(State(hub): State<Hub>, ws: WebSocketUpgrade) -> Response {
    ws.max_message_size(512 * 1024).on_upgrade(move |socket| serve_link(hub, socket))
}

// 名乗りあり　鍵を確かめ　門開く
async fn serve_link(hub: Hub, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let hello = match tokio::time::timeout(Duration::from_secs(10), stream.next()).await {
        Ok(Some(Ok(Message::Text(t)))) => serde_json::from_str::<Up>(&t).ok(),
        _ => None,
    };
    let Some(Up::Hello { name, token, key, version, open }) = hello else { return };
    let reason = if !same(&token, &hub.token) {
        Some("bad token")
    } else if !valid_name(&name) {
        Some("bad name")
    } else if version != PROTOCOL {
        Some("protocol mismatch")
    } else {
        None
    };
    if let Some(reason) = reason {
        tracing::warn!("station {name:?} rejected: {reason}");
        let _ = sink.send(text(&Down::Reject { reason: reason.into() })).await;
        return;
    }
    if sink.send(text(&Down::Welcome)).await.is_err() {
        return;
    }

    let (tx, mut rx) = mpsc::unbounded_channel();
    let link = Arc::new(Link {
        key,
        open,
        tx,
        pending: Mutex::default(),
        next: AtomicU32::new(1),
        library: Mutex::default(),
        pool: Mutex::default(),
        warming: AtomicBool::new(false),
    });
    if let Some(old) = hub.stations.0.lock().unwrap().insert(name.clone(), link.clone()) {
        let _ = old.tx.send(Message::Close(None));
    }
    tracing::info!("station {name} linked{}", if open { " (open)" } else { "" });

    let writer = tokio::spawn(async move {
        while let Some(m) = rx.recv().await {
            let closing = matches!(m, Message::Close(_));
            if sink.send(m).await.is_err() || closing {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        match msg {
            Message::Text(t) => match serde_json::from_str::<Up>(&t) {
                Ok(Up::Head { id, status, headers }) => link.on_head(id, Head { status, headers }),
                Ok(Up::End { id }) => {
                    link.pending.lock().unwrap().remove(&id);
                }
                Ok(Up::Changed) => link.forget(),
                _ => {}
            },
            Message::Binary(b) => {
                if let Some((id, _)) = unframe(&b) {
                    link.on_chunk(id, b.slice(4..));
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // 糸切れて　帳の名をば　消しにけり
    {
        let mut map = hub.stations.0.lock().unwrap();
        if map.get(&name).is_some_and(|l| Arc::ptr_eq(l, &link)) {
            map.remove(&name);
        }
    }
    link.pending.lock().unwrap().clear();
    writer.abort();
    tracing::info!("station {name} gone");
}
