mod soundpack;
mod util;

use axum::{Router, routing::post};
use clap::Parser;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use util::signal::shutdown_signal;
use util::state::{AppState, Mutable};

use util::Args;
use util::playback::{get_output_stream, list_host_devices};

use anyhow::{Context, Result};
use soundpack::Preset;
use util::handler::update;

const DEFAULT_LOG_LEVEL: LevelFilter = if cfg!(debug_assertions) {
    LevelFilter::DEBUG
} else {
    LevelFilter::INFO
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(DEFAULT_LOG_LEVEL.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let args = Args::parse();

    if args.list_devices {
        list_host_devices();
        return Ok(());
    }

    // initialize the specified audio device
    let output_stream = get_output_stream(&args.device).context("failed to get output stream")?;
    let preset = Preset::try_from(args.preset.as_ref())
        .with_context(|| format!("failed to parse preset '{}'", &args.preset))?;
    info!("preset '{}' loaded successfully", &args.preset);
    info!("variant: {}", args.variant.as_deref().unwrap_or("none"));

    let app_state = Arc::new(AppState {
        mutable: RwLock::new(Mutable {
            steamid: "".into(),
            ply_kills: 0,
            ply_hs_kills: 0,
        }),
        stream_handle: output_stream.1,
        args,
        preset,
    });

    let app = Router::new()
        .route("/", post(update))
        .with_state(app_state)
        .layer((
            TraceLayer::new_for_http(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
