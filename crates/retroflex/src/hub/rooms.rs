use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use data_encoding::BASE64URL_NOPAD;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::Hub;
use super::stations::Stations;
use crate::proto::Track;
use crate::util::{key_hash, now_ms, same, token};

const LEAD_MS: f64 = 450.0;
const GRACE: Duration = Duration::from_secs(300);
const BACKLOG: usize = 200;
const MAX_ROOMS: usize = 1000;
const MAX_QUEUE: usize = 5000;
const MAX_MEMBERS: usize = 64;
const AUTO: usize = 3;
const CHAT_CHARS: usize = 450;
const CHAT_WINDOW: Duration = Duration::from_secs(60);
const DRAFT_GAP: Duration = Duration::from_millis(25);
const SEALED: usize = 40;
const PUBLIC_CAP: usize = 12;
const PUBLIC_AHEAD: usize = 10;
const PUBLIC_EACH: usize = 2;

// 曲の影　名と長さだけ　写し取る
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snap {
    id: String,
    title: String,
    artist: String,
    #[serde(default)]
    album: String,
    #[serde(default)]
    duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cover: Option<String>,
}

impl From<&Track> for Snap {
    fn from(t: &Track) -> Self {
        Snap {
            id: t.id.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            duration: t.duration,
            cover: t.cover.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Item {
    qid: u32,
    by: u32,
    #[serde(skip_serializing_if = "is_false")]
    auto: bool,
    #[serde(flatten)]
    track: Snap,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Default, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    #[default]
    End,
    Next,
    Now,
}

// 部屋の声　十余りの　型に分く
#[derive(Debug, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
enum In {
    Ping { c: f64 },
    Profile { ct: String },
    Chat { ct: String },
    Draft { ct: String },
    Enqueue {
        tracks: Vec<Snap>,
        #[serde(default)]
        mode: Mode,
    },
    Remove { qid: u32 },
    Move { qid: u32, to: usize },
    Play { qid: Option<u32>, pos: Option<f64> },
    Pause,
    Seek { pos: f64 },
    Next,
    Prev,
    Ended { qid: u32 },
}

// 封の丈　四つずつ数え　字を量る
fn sealed_chars(ct: &str) -> Option<usize> {
    let raw = BASE64URL_NOPAD.decode(ct.as_bytes()).ok()?;
    let body = raw.len().checked_sub(SEALED)?;
    (body % 4 == 0).then_some(body / 4)
}

struct Member {
    profile: Option<String>,
    tx: mpsc::Sender<Message>,
    spent: VecDeque<(Instant, usize)>,
    drafted: Option<Instant>,
}

impl Member {
    fn new(tx: mpsc::Sender<Message>) -> Self {
        Self { profile: None, tx, spent: VecDeque::new(), drafted: None }
    }

    // 一分に　語り得る字の　升を見る
    fn spend(&mut self, chars: usize, now: Instant) -> Result<(), f64> {
        while self.spent.front().is_some_and(|(t, _)| now.duration_since(*t) > CHAT_WINDOW) {
            self.spent.pop_front();
        }
        let used: usize = self.spent.iter().map(|(_, n)| n).sum();
        if used + chars <= CHAT_CHARS {
            self.spent.push_back((now, chars));
            return Ok(());
        }
        let mut freed = used + chars - CHAT_CHARS;
        for (t, n) in &self.spent {
            if *n >= freed {
                return Err(CHAT_WINDOW.saturating_sub(now.duration_since(*t)).as_secs_f64());
            }
            freed -= n;
        }
        Err(CHAT_WINDOW.as_secs_f64())
    }
}

// 誰もみな　同じ盤面　同じ針
#[derive(Default)]
struct Inner {
    members: BTreeMap<u32, Member>,
    next_member: u32,
    next_qid: u32,
    queue: Vec<Item>,
    cur: Option<u32>,
    playing: bool,
    pos: f64,
    at: f64,
    rev: u64,
    backlog: VecDeque<Value>,
    empty_since: Option<Instant>,
    starved: bool,
    public: bool,
    skip: HashSet<u32>,
}

fn msg(v: &Value) -> Message {
    Message::Text(v.to_string().into())
}

impl Inner {
    // 針の位置　時の流れに　添わせ読む
    fn position(&self, now: f64) -> f64 {
        if self.playing && now > self.at { self.pos + (now - self.at) / 1000.0 } else { self.pos }
    }

    fn index_of(&self, qid: u32) -> Option<usize> {
        self.queue.iter().position(|i| i.qid == qid)
    }

    fn current(&self) -> Option<&Item> {
        self.cur.and_then(|q| self.index_of(q)).map(|i| &self.queue[i])
    }

    fn ahead(&self) -> &[Item] {
        match self.cur.and_then(|q| self.index_of(q)) {
            Some(i) => &self.queue[i + 1..],
            None => &self.queue[..],
        }
    }

    // 少し先　皆で揃えて　針落とす
    fn start(&mut self, qid: Option<u32>, pos: f64, now: f64) {
        self.cur = qid;
        self.pos = pos.max(0.0);
        self.playing = qid.is_some();
        self.at = now + LEAD_MS;
        self.skip.clear();
    }

    fn advance(&mut self, step: isize, now: f64) {
        let idx = self.cur.and_then(|q| self.index_of(q));
        let next = match idx {
            Some(i) => i as isize + step,
            None if step > 0 => 0,
            None => return,
        };
        if next >= 0 && (next as usize) < self.queue.len() {
            let qid = self.queue[next as usize].qid;
            self.start(Some(qid), 0.0, now);
        } else if step < 0 {
            let cur = self.cur;
            self.start(cur, 0.0, now);
        } else {
            self.cur = None;
            self.playing = false;
            self.pos = 0.0;
            self.at = now;
            self.skip.clear();
            self.starved = !self.queue.is_empty();
        }
    }

    // 誰も来ず　籤で三つの　歌を足す
    fn refill(&mut self, pool: &[Snap], now: f64) {
        if !self.starved || self.members.is_empty() || pool.is_empty() {
            return;
        }
        let picks: Vec<Snap> = {
            let recent: HashSet<&str> = self.queue.iter().rev().take(40).map(|i| i.track.id.as_str()).collect();
            let mut bag: Vec<&Snap> = pool.iter().filter(|t| !recent.contains(t.id.as_str())).collect();
            if bag.len() < AUTO {
                bag = pool.iter().collect();
            }
            let mut picks = Vec::new();
            while picks.len() < AUTO && !bag.is_empty() {
                let i = getrandom::u32().unwrap_or(0) as usize % bag.len();
                picks.push(bag.swap_remove(i).clone());
            }
            picks
        };
        self.starved = false;
        let first = self.next_qid + 1;
        for track in picks {
            self.next_qid += 1;
            self.queue.push(Item { qid: self.next_qid, by: 0, auto: true, track });
        }
        if self.queue.len() > MAX_QUEUE {
            let extra = self.queue.len() - MAX_QUEUE;
            self.queue.drain(..extra);
        }
        self.start(Some(first), 0.0, now);
        self.changed(now);
    }

    fn state(&self, now: f64) -> Value {
        json!({
            "t": "state",
            "rev": self.rev,
            "queue": self.queue,
            "cur": self.cur,
            "playing": self.playing,
            "pos": self.pos,
            "at": self.at,
            "now": now,
            "public": self.public,
            "skip": self.skip.len(),
        })
    }

    // 部屋中に　ひとつの声を　配りけり
    fn broadcast(&self, m: &Value, except: Option<u32>) {
        let frame = msg(m);
        for (id, member) in &self.members {
            if Some(*id) != except {
                let _ = member.tx.try_send(frame.clone());
            }
        }
    }

    fn tell(&self, to: u32, m: &Value) {
        if let Some(member) = self.members.get(&to) {
            let _ = member.tx.try_send(msg(m));
        }
    }

    fn changed(&mut self, now: f64) {
        self.rev += 1;
        let s = self.state(now);
        self.broadcast(&s, None);
    }

    // 広間では　一人二つと　十まで積む
    fn admit(&self, me: u32, tracks: Vec<Snap>) -> Vec<Snap> {
        let room = PUBLIC_AHEAD.saturating_sub(self.ahead().len());
        let mine = self.ahead().iter().filter(|i| i.by == me).count();
        let allowed = room.min(PUBLIC_EACH.saturating_sub(mine));
        tracks.into_iter().take(allowed).collect()
    }

    // 票の数　半ばを越せば　次へ行く
    fn tally(&mut self, now: f64) {
        if !self.skip.is_empty() && self.skip.len() * 2 > self.members.len() {
            self.advance(1, now);
        }
        self.changed(now);
    }

    // 平らかに　誰の頼みも　同じ重さ
    fn handle(&mut self, me: u32, cmd: In, now: f64) {
        let public = self.public;
        match cmd {
            In::Ping { .. } => {}
            In::Profile { ct } if ct.len() <= 2048 => {
                if let Some(m) = self.members.get_mut(&me) {
                    m.profile = Some(ct.clone());
                }
                self.broadcast(&json!({ "t": "profile", "id": me, "ct": ct }), None);
            }
            In::Chat { ct } => {
                let Some(chars) = sealed_chars(&ct).filter(|n| (1..=CHAT_CHARS).contains(n)) else {
                    self.tell(me, &json!({ "t": "limit", "wait": 0 }));
                    return;
                };
                let Some(member) = self.members.get_mut(&me) else { return };
                if let Err(wait) = member.spend(chars, Instant::now()) {
                    self.tell(me, &json!({ "t": "limit", "wait": wait.ceil() }));
                    return;
                }
                let line = json!({ "t": "chat", "from": me, "ct": ct, "at": now });
                self.backlog.push_back(line.clone());
                while self.backlog.len() > BACKLOG {
                    self.backlog.pop_front();
                }
                self.broadcast(&line, None);
            }
            In::Draft { ct } => {
                if !ct.is_empty() {
                    if sealed_chars(&ct).is_none_or(|n| n > CHAT_CHARS) {
                        return;
                    }
                    let Some(member) = self.members.get_mut(&me) else { return };
                    let at = Instant::now();
                    if member.drafted.is_some_and(|t| at.duration_since(t) < DRAFT_GAP) {
                        return;
                    }
                    member.drafted = Some(at);
                }
                self.broadcast(&json!({ "t": "draft", "from": me, "ct": ct }), Some(me));
            }
            In::Enqueue { tracks, mode } => {
                let (tracks, mode) = if public { (self.admit(me, tracks), Mode::End) } else { (tracks, mode) };
                let room = MAX_QUEUE.saturating_sub(self.queue.len());
                let items: Vec<Item> = tracks
                    .into_iter()
                    .take(room)
                    .map(|track| {
                        self.next_qid += 1;
                        Item { qid: self.next_qid, by: me, auto: false, track }
                    })
                    .collect();
                let Some(first) = items.first().map(|i| i.qid) else { return self.changed(now) };
                let at = match (mode, self.cur.and_then(|q| self.index_of(q))) {
                    (Mode::Next | Mode::Now, Some(i)) => i + 1,
                    _ => self.queue.len(),
                };
                self.queue.splice(at..at, items);
                self.starved = false;
                if mode == Mode::Now || self.cur.is_none() {
                    self.start(Some(first), 0.0, now);
                }
                self.changed(now);
            }
            In::Remove { qid } => {
                let Some(i) = self.index_of(qid) else { return };
                if public && (self.cur == Some(qid) || !(self.queue[i].by == me || self.queue[i].auto)) {
                    return self.changed(now);
                }
                if self.cur == Some(qid) {
                    let was_playing = self.playing;
                    self.advance(1, now);
                    if !was_playing && self.cur.is_some() {
                        self.playing = false;
                        self.at = now;
                    }
                }
                self.queue.remove(i);
                self.starved = false;
                self.changed(now);
            }
            In::Move { qid, to } if !public => {
                let Some(i) = self.index_of(qid) else { return };
                let item = self.queue.remove(i);
                let to = to.min(self.queue.len());
                self.queue.insert(to, item);
                self.changed(now);
            }
            In::Play { .. } if public && self.cur.is_some() => self.changed(now),
            In::Play { qid, pos } => {
                match qid.filter(|q| self.index_of(*q).is_some()) {
                    Some(q) => self.start(Some(q), pos.unwrap_or(0.0), now),
                    None if self.cur.is_some() => {
                        let p = pos.unwrap_or_else(|| self.position(now));
                        let cur = self.cur;
                        self.start(cur, p, now);
                    }
                    None => self.advance(1, now),
                }
                self.changed(now);
            }
            In::Pause | In::Seek { .. } | In::Prev if public => self.changed(now),
            In::Pause => {
                if self.playing {
                    self.pos = self.position(now);
                    self.playing = false;
                    self.at = now;
                    self.changed(now);
                }
            }
            In::Seek { pos } => {
                let Some(dur) = self.current().map(|i| i.track.duration) else { return };
                self.pos = if dur > 0.0 { pos.clamp(0.0, dur) } else { pos.max(0.0) };
                self.at = now + if self.playing { LEAD_MS } else { 0.0 };
                self.changed(now);
            }
            In::Next if public => {
                self.skip.insert(me);
                self.tally(now);
            }
            In::Next => {
                self.advance(1, now);
                self.changed(now);
            }
            In::Prev => {
                if self.position(now) > 3.0 {
                    let cur = self.cur;
                    self.start(cur, 0.0, now);
                } else {
                    self.advance(-1, now);
                }
                self.changed(now);
            }
            In::Ended { qid } => {
                let near_end = self.current().is_some_and(|i| self.position(now) >= i.track.duration - 4.0);
                if self.cur == Some(qid) && self.playing && near_end {
                    self.advance(1, now);
                    self.changed(now);
                }
            }
            _ => {}
        }
    }
}

// 合言葉　知る者だけの　小部屋なり
pub struct Room {
    station: String,
    ticket: String,
    inner: Mutex<Inner>,
}

#[derive(Clone, Default)]
pub struct Rooms(Arc<Mutex<HashMap<String, Arc<Room>>>>);

impl Rooms {
    fn get(&self, id: &str) -> Option<Arc<Room>> {
        self.0.lock().unwrap().get(id).cloned()
    }

    // 札を見せ　この駅の歌　聴く許し
    pub fn admits(&self, id: &str, ticket: &str, station: &str) -> bool {
        self.get(id).is_some_and(|r| r.station == station && same(&key_hash(ticket), &r.ticket))
    }

    // 人去りて　しばし待てども　灯は消ゆ
    pub fn sweep(&self) {
        self.0.lock().unwrap().retain(|_, r| {
            let inner = r.inner.lock().unwrap();
            !inner.empty_since.is_some_and(|t| t.elapsed() > GRACE)
        });
    }

    // 誰も来ず　針の終わりは　部屋が知る
    pub fn tick(&self, stations: &Stations) {
        let now = now_ms();
        for room in self.0.lock().unwrap().values() {
            let mut inner = room.inner.lock().unwrap();
            let over = inner.playing
                && inner.current().is_some_and(|i| i.track.duration > 0.0 && inner.position(now) > i.track.duration + 2.0);
            if over {
                inner.advance(1, now);
                inner.changed(now);
            }
            if inner.starved && !inner.members.is_empty() {
                feed(&mut inner, &room.station, stations, now);
            }
        }
    }
}

// 籤箱の　中身がなくば　取り寄せて
fn feed(inner: &mut Inner, station: &str, stations: &Stations, now: f64) {
    let Some(link) = stations.get(station) else { return };
    match link.pool() {
        Some(pool) => inner.refill(&pool, now),
        None => link.warm(),
    }
}

#[derive(Deserialize)]
pub struct CreateReq {
    station: String,
    #[serde(default)]
    key: String,
    ticket: String,
}

// 駅の鍵　持つ者ひとり　部屋を開く
pub async fn create(State(hub): State<Hub>, Json(req): Json<CreateReq>) -> Response {
    let Some(link) = hub.stations.get(&req.station) else { return StatusCode::SERVICE_UNAVAILABLE.into_response() };
    let keyless = link.open && req.key.is_empty();
    if !keyless && !same(&key_hash(&req.key), &link.key) {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !(16..=128).contains(&req.ticket.len()) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let mut rooms = hub.rooms.0.lock().unwrap();
    if rooms.len() >= MAX_ROOMS {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let id = loop {
        let id = token(5);
        if !rooms.contains_key(&id) {
            break id;
        }
    };
    let inner = Inner { empty_since: Some(Instant::now()), ..Default::default() };
    let room = Room { station: req.station, ticket: key_hash(&req.ticket), inner: Mutex::new(inner) };
    rooms.insert(id.clone(), Arc::new(room));
    tracing::info!("room {id} opened");
    Json(json!({ "id": id })).into_response()
}

#[derive(Deserialize)]
pub struct JoinQuery {
    t: String,
}

// 戸の外で　まだ灯りあるかと　耳澄ます
pub async fn probe(State(hub): State<Hub>, Path(id): Path<String>, Query(q): Query<JoinQuery>) -> StatusCode {
    match hub.rooms.get(&id) {
        Some(room) if same(&key_hash(&q.t), &room.ticket) => StatusCode::NO_CONTENT,
        Some(_) => StatusCode::FORBIDDEN,
        None => StatusCode::NOT_FOUND,
    }
}

pub async fn join(State(hub): State<Hub>, Path(id): Path<String>, Query(q): Query<JoinQuery>, ws: WebSocketUpgrade) -> Response {
    match hub.rooms.get(&id) {
        Some(room) if same(&key_hash(&q.t), &room.ticket) => {
            ws.max_message_size(64 * 1024).on_upgrade(move |socket| member(room, hub.stations, socket))
        }
        Some(_) => StatusCode::FORBIDDEN.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// 戸を叩き　名もなき席に　腰おろす
async fn member(room: Arc<Room>, stations: Stations, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(512);
    let seat = {
        let mut inner = room.inner.lock().unwrap();
        let cap = if inner.public { PUBLIC_CAP } else { MAX_MEMBERS };
        (inner.members.len() < cap).then(|| {
            inner.next_member += 1;
            let me = inner.next_member;
            let now = now_ms();
            let members: Vec<Value> = inner.members.iter().map(|(id, m)| json!({ "id": id, "ct": m.profile })).collect();
            let backlog: Vec<&Value> = inner.backlog.iter().collect();
            let welcome = json!({ "t": "welcome", "you": me, "now": now, "station": room.station, "public": inner.public, "members": members, "backlog": backlog });
            let _ = tx.try_send(msg(&welcome));
            let _ = tx.try_send(msg(&inner.state(now)));
            inner.broadcast(&json!({ "t": "join", "id": me }), None);
            inner.members.insert(me, Member::new(tx.clone()));
            inner.empty_since = None;
            me
        })
    };
    let Some(me) = seat else {
        let _ = sink.send(msg(&json!({ "t": "full" }))).await;
        return;
    };

    let writer = tokio::spawn(async move {
        let mut beat = tokio::time::interval(Duration::from_secs(25));
        loop {
            tokio::select! {
                m = rx.recv() => match m {
                    Some(m) => if sink.send(m).await.is_err() { break },
                    None => break,
                },
                _ = beat.tick() => if sink.send(Message::Ping(Default::default())).await.is_err() { break },
            }
        }
    });

    while let Some(Ok(m)) = stream.next().await {
        let Message::Text(t) = m else {
            if matches!(m, Message::Close(_)) {
                break;
            }
            continue;
        };
        let Ok(cmd) = serde_json::from_str::<In>(&t) else { continue };
        if let In::Ping { c } = cmd {
            let _ = tx.try_send(msg(&json!({ "t": "pong", "c": c, "s": now_ms() })));
            continue;
        }
        let now = now_ms();
        let mut inner = room.inner.lock().unwrap();
        inner.handle(me, cmd, now);
        if inner.starved {
            feed(&mut inner, &room.station, &stations, now);
        }
    }

    // 席を立ち　椅子の温みも　やがて消ゆ
    {
        let mut inner = room.inner.lock().unwrap();
        inner.members.remove(&me);
        inner.broadcast(&json!({ "t": "leave", "id": me }), None);
        if inner.skip.remove(&me) || inner.public && !inner.skip.is_empty() {
            inner.tally(now_ms());
        }
        if inner.members.is_empty() {
            inner.empty_since = Some(Instant::now());
        }
    }
    writer.abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(id: &str, duration: f64) -> Snap {
        Snap { id: id.into(), title: id.into(), artist: "a".into(), album: "b".into(), duration, cover: None }
    }

    fn seat(r: &mut Inner, id: u32) {
        let (tx, _rx) = mpsc::channel(64);
        r.members.insert(id, Member::new(tx));
    }

    fn sealed(chars: usize) -> String {
        BASE64URL_NOPAD.encode(&vec![7u8; SEALED + 4 * chars])
    }

    // 並べては　先へ後へと　針移す
    #[test]
    fn queue_and_transport() {
        let mut r = Inner::default();
        r.handle(1, In::Enqueue { tracks: vec![snap("x", 100.0), snap("y", 50.0)], mode: Mode::End }, 0.0);
        assert_eq!(r.cur, Some(1));
        assert!(r.playing);
        assert_eq!(r.at, LEAD_MS);
        assert!((r.position(LEAD_MS + 2000.0) - 2.0).abs() < 1e-9);

        r.handle(1, In::Enqueue { tracks: vec![snap("z", 10.0)], mode: Mode::Next }, 1000.0);
        assert_eq!(r.queue.iter().map(|i| i.track.id.as_str()).collect::<Vec<_>>(), ["x", "z", "y"]);

        r.handle(2, In::Pause, LEAD_MS + 5000.0);
        assert!(!r.playing);
        assert!((r.pos - 5.0).abs() < 1e-9);

        r.handle(2, In::Next, 10_000.0);
        assert_eq!(r.current().unwrap().track.id, "z");

        r.handle(2, In::Ended { qid: 3 }, 10_000.0 + LEAD_MS + 1000.0);
        assert_eq!(r.current().unwrap().track.id, "z");
        r.handle(2, In::Ended { qid: 3 }, 10_000.0 + LEAD_MS + 9000.0);
        assert_eq!(r.current().unwrap().track.id, "y");

        r.handle(1, In::Remove { qid: 2 }, 30_000.0);
        assert_eq!(r.cur, None);
        assert!(!r.playing);
    }

    // 歌尽きて　籤の三つが　また鳴りぬ
    #[test]
    fn refills_when_the_queue_runs_dry() {
        let mut r = Inner::default();
        seat(&mut r, 1);
        let pool: Vec<Snap> = (0..10).map(|i| snap(&format!("p{i}"), 60.0)).collect();
        r.handle(1, In::Enqueue { tracks: vec![snap("x", 10.0)], mode: Mode::End }, 0.0);
        r.handle(1, In::Next, 1000.0);
        assert!(r.starved);
        r.refill(&pool, 1000.0);
        assert!(!r.starved);
        assert_eq!(r.queue.len(), 4);
        assert!(r.queue[1..].iter().all(|i| i.auto && i.by == 0));
        assert_eq!(r.cur, Some(r.queue[1].qid));
        assert!(r.playing);

        let last = r.queue[3].qid;
        r.handle(1, In::Play { qid: Some(last), pos: None }, 2000.0);
        r.handle(1, In::Remove { qid: last }, 3000.0);
        assert!(!r.starved);
        r.refill(&pool, 3000.0);
        assert_eq!(r.queue.len(), 3);
    }

    // 広間では　止めず戻らず　票で飛ぶ
    #[test]
    fn public_rules() {
        let mut r = Inner { public: true, ..Default::default() };
        for id in 1..=3 {
            seat(&mut r, id);
        }
        let many: Vec<Snap> = (0..5).map(|i| snap(&format!("a{i}"), 100.0)).collect();
        r.handle(1, In::Enqueue { tracks: many, mode: Mode::Now }, 0.0);
        assert_eq!(r.queue.len(), 2);
        r.handle(1, In::Pause, 1000.0);
        assert!(r.playing);
        r.handle(1, In::Seek { pos: 50.0 }, 1000.0);
        assert_eq!(r.pos, 0.0);

        let cur = r.cur;
        r.handle(2, In::Next, 2000.0);
        assert_eq!(r.cur, cur);
        r.handle(3, In::Next, 2000.0);
        assert_ne!(r.cur, cur);
        assert!(r.skip.is_empty());

        r.handle(2, In::Enqueue { tracks: vec![snap("b", 10.0)], mode: Mode::End }, 3000.0);
        let theirs = r.queue.iter().find(|i| i.by == 2).unwrap().qid;
        r.handle(3, In::Remove { qid: theirs }, 3000.0);
        assert!(r.index_of(theirs).is_some());
        r.handle(2, In::Remove { qid: theirs }, 3000.0);
        assert!(r.index_of(theirs).is_none());
    }

    // 一分に　四百五十字　越えるなと
    #[test]
    fn chat_budget() {
        let mut r = Inner::default();
        seat(&mut r, 1);
        r.handle(1, In::Chat { ct: sealed(300) }, 0.0);
        assert_eq!(r.backlog.len(), 1);
        r.handle(1, In::Chat { ct: sealed(200) }, 0.0);
        assert_eq!(r.backlog.len(), 1);
        r.handle(1, In::Chat { ct: sealed(150) }, 0.0);
        assert_eq!(r.backlog.len(), 2);
        r.handle(1, In::Chat { ct: BASE64URL_NOPAD.encode(&[1u8; SEALED + 6]) }, 0.0);
        assert_eq!(r.backlog.len(), 2);
        assert_eq!(sealed_chars(&sealed(12)), Some(12));
    }
}
