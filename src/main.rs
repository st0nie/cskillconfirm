use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use axum::{extract::State, routing::post, Json, Router};
use clap::Parser;
use gsi_cs2::Body;
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// sound preset to use (available: "varolant")
    #[arg(short, long, default_value = "varolant")]
    preset: String,
}

struct AppState {
    ply_name: String,
    ply_kills: u16,
    preset: String,
}

async fn update(State(app_state): State<Arc<Mutex<AppState>>>, data: Json<Body>) {
    let map = data.map.as_ref();
    if let None = map {
        return;
    }

    let player_data = data.player.as_ref();
    if let None = player_data {
        return;
    }

    let player = player_data.unwrap();
    let state = player.state.as_ref().unwrap();

    let mut app_state = app_state.lock().unwrap();

    let current_kills = state.round_kills;
    let original_kills = app_state.ply_kills;

    let current_name = player.name.as_ref().unwrap();
    let original_name = &app_state.ply_name;

    if current_kills == original_kills + 1 && (current_name == original_name || original_name == "")
    {
        let sound_num = if current_kills > 5 { 5 } else { current_kills };
        let preset = app_state.preset.to_string();

        thread::spawn(move || {
            let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let file = File::open(format!("sounds/{}/{}.mp3", preset, sound_num)).unwrap();
            let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            sink.append(source);
            sink.set_volume(0.3);
            sink.play();
            sink.sleep_until_end();
        });
        tracing::info!("player:{} kills:{}", current_name, current_kills);
    }

    app_state.ply_kills = current_kills;
    app_state.ply_name = current_name.to_string();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

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

    let app_state = Arc::new(Mutex::new(AppState {
        ply_name: "".to_string(),
        ply_kills: 0,
        preset: args.preset,
    }));

    let app = Router::new()
        .route("/", post(update))
        .with_state(app_state.clone())
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
