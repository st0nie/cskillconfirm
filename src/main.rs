mod soundpack;
mod utils;

use axum::{routing::post, Router};
use clap::Parser;
use rodio::OutputStreamHandle;
use soundpack::preset;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use utils::args::Args;
use utils::playback::{get_output_stream, list_host_devices};

use soundpack::preset::Preset;
use utils::handler::{shutdown_signal, update};

struct AppState {
    ply_name: String,
    ply_kills: u16,
    stream_handle: OutputStreamHandle,
    args: Args,
    preset: Preset,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let args = Args::parse();

    if args.list_devices {
        list_host_devices();
        return;
    }

    // initialize the specified audio device
    let output_stream = get_output_stream(&args.device);
    let preset = preset::parse_from_name(&args.preset).unwrap_or_else(|e| {
        error!("failed to parse preset '{}': {}", &args.preset, e);
        std::process::exit(1);
    });
    info!("preset '{}' loaded successfully", &args.preset);
    info!("{:?}", preset);
    info!("variant: {}", args.variant.as_deref().unwrap_or("none"));

    let app_state = Arc::new(Mutex::new(AppState {
        ply_name: "".into(),
        ply_kills: 0,
        stream_handle: output_stream.1,
        args,
        preset,
    }));

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
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
