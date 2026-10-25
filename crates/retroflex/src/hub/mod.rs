mod assets;
mod proxy;
mod rooms;
mod stations;
mod translate;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::Router;
use axum::routing::{get, post};
use clap::Args;
use tower_http::cors::CorsLayer;

use rooms::{Invites, Rooms};
use stations::Stations;
use translate::Translator;

#[derive(Args)]
pub struct HubArgs {
    #[arg(long, default_value = "127.0.0.1:7746", env = "RETROFLEX_LISTEN", help = "Address to listen on")]
    listen: SocketAddr,
    #[arg(long, env = "RETROFLEX_HUB_TOKEN", hide_env_values = true, help = "Token stations must present")]
    token: String,
}

// 駅と部屋　ふたつの帳を　ひと手にて
#[derive(Clone)]
pub struct Hub {
    stations: Stations,
    rooms: Rooms,
    invites: Invites,
    translator: Translator,
    token: Arc<str>,
}

// 辻に立ち　行き交う声を　繋ぐのみ
pub async fn run(a: HubArgs) -> Result<()> {
    let hub = Hub { stations: Stations::default(), rooms: Rooms::default(), invites: Invites::default(), translator: Translator::default(), token: a.token.into() };

    let clock = hub.rooms.clone();
    let stations = hub.stations.clone();
    let invites = hub.invites.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        for n in 0u64.. {
            tick.tick().await;
            clock.tick(&stations);
            if n % 30 == 0 {
                clock.sweep();
                invites.sweep();
            }
        }
    });

    let app = Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/uplink", get(stations::uplink))
        .route("/api/stations", get(proxy::open))
        .route("/api/s/{station}/library", get(proxy::library))
        .route("/api/s/{station}/audio/{track}", get(proxy::audio))
        .route("/api/s/{station}/cover/{cover}", get(proxy::cover))
        .route("/api/s/{station}/lyrics/{track}", get(proxy::lyrics))
        .route("/api/s/{station}/lyrics/{track}/to/{lang}", get(translate::lyrics))
        .route("/api/rooms", post(rooms::create))
        .route("/api/rooms/{id}", get(rooms::probe))
        .route("/api/rooms/{id}/ws", get(rooms::join))
        .route("/api/match", post(rooms::matchmake))
        .route("/api/invites", post(rooms::invite))
        .route("/api/invites/{code}", get(rooms::redeem))
        .fallback(assets::serve)
        .layer(CorsLayer::permissive())
        .with_state(hub);

    let listener = tokio::net::TcpListener::bind(a.listen).await?;
    tracing::info!("hub listening on http://{}", a.listen);
    axum::serve(listener, app).await?;
    Ok(())
}
