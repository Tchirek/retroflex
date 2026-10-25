use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use super::Hub;
use super::stations::Link;
use crate::proto::Op;
use crate::util::{key_hash, same};

#[derive(Deserialize)]
pub struct Pass {
    k: Option<String>,
    #[serde(default)]
    thumb: Option<u8>,
}

// 合鍵か　部屋の札かを　改める
fn authorize(hub: &Hub, station: &str, k: Option<&str>) -> Result<Arc<Link>, StatusCode> {
    let k = k.unwrap_or("");
    let link = hub.stations.get(station);
    if let Some((room, ticket)) = k.split_once('.') {
        if hub.rooms.admits(room, ticket, station) {
            return link.ok_or(StatusCode::SERVICE_UNAVAILABLE);
        }
        return Err(StatusCode::FORBIDDEN);
    }
    match link {
        Some(l) if l.open && k.is_empty() => Ok(l),
        Some(l) if !k.is_empty() && same(&key_hash(k), &l.key) => Ok(l),
        _ if k.is_empty() => Err(StatusCode::UNAUTHORIZED),
        _ => Err(StatusCode::FORBIDDEN),
    }
}

// 戸を開けて　誰でも寄れる　駅の名を
pub async fn open(State(hub): State<Hub>) -> Response {
    ([(header::CACHE_CONTROL, "no-store")], axum::Json(hub.stations.open())).into_response()
}

// 目録は　一度借りれば　手元に置く
pub async fn library(State(hub): State<Hub>, Path(station): Path<String>, Query(p): Query<Pass>) -> Response {
    let link = match authorize(&hub, &station, p.k.as_deref()) {
        Ok(l) => l,
        Err(code) => return code.into_response(),
    };
    match link.catalog().await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")], bytes).into_response(),
        Err(code) => code.into_response(),
    }
}

// 駅からの　水をそのまま　管に注ぐ
async fn relay(link: Arc<Link>, op: Op, range: Option<String>, cache: &'static str) -> Response {
    let (head, body) = match link.request(op, range).await {
        Ok(r) => r,
        Err(code) => return code.into_response(),
    };
    let mut resp = Response::builder().status(head.status);
    for (k, v) in &head.headers {
        resp = resp.header(k.as_str(), v.as_str());
    }
    if head.status < 300 {
        resp = resp.header(header::CACHE_CONTROL, cache);
    }
    resp.body(Body::from_stream(body)).unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

fn range(headers: &HeaderMap) -> Option<String> {
    headers.get(header::RANGE).and_then(|v| v.to_str().ok()).map(String::from)
}

pub async fn audio(
    State(hub): State<Hub>,
    Path((station, track)): Path<(String, String)>,
    Query(p): Query<Pass>,
    headers: HeaderMap,
) -> Response {
    match authorize(&hub, &station, p.k.as_deref()) {
        Ok(link) => relay(link, Op::Audio { track }, range(&headers), "private, max-age=86400").await,
        Err(code) => code.into_response(),
    }
}

pub async fn cover(State(hub): State<Hub>, Path((station, cover)): Path<(String, String)>, Query(p): Query<Pass>) -> Response {
    let thumb = p.thumb.unwrap_or(0) == 1;
    match authorize(&hub, &station, p.k.as_deref()) {
        Ok(link) => relay(link, Op::Cover { cover, thumb }, None, "private, max-age=604800, immutable").await,
        Err(code) => code.into_response(),
    }
}

pub async fn lyrics(State(hub): State<Hub>, Path((station, track)): Path<(String, String)>, Query(p): Query<Pass>) -> Response {
    match authorize(&hub, &station, p.k.as_deref()) {
        Ok(link) => relay(link, Op::Lyrics { track }, None, "private, max-age=3600").await,
        Err(code) => code.into_response(),
    }
}
