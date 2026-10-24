mod hub;
mod proto;
mod station;
mod util;

use std::io::IsTerminal;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

// 一つの名　二つの貌を　持つ笛よ
#[derive(Parser)]
#[command(name = "retroflex", version, about = "Listen together, without a center.")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    #[command(about = "Run a hub: rooms, clock and relay")]
    Hub(hub::HubArgs),
    #[command(about = "Share a music library through a hub")]
    Station(station::StationArgs),
}

// 灯をともし　ささやく声を　書き留める
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,lofty=error")))
        .with_ansi(std::io::stdout().is_terminal())
        .init();
    match Cli::parse().cmd {
        Cmd::Hub(a) => hub::run(a).await,
        Cmd::Station(a) => station::run(a).await,
    }
}
