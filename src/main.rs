use axum::{extract::State, routing::post, Json, Router};
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use gsi_cs2::Body;
use rodio::{OutputStream, OutputStreamHandle};
use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::signal;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{self, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// list all available audio devices
    #[arg(short, long, default_value = "false")]
    list_devices: bool,
    /// sound preset to use (available: "varolant", "crossfire")
    #[arg(short, long, default_value = "varolant")]
    preset: String,
    /// select output device
    #[arg(short, long, default_value = "default")]
    device: String,
    ///
    #[arg(short, long, default_value = "1.0")]
    volume: f32,
    /// disable voice for some presets
    #[arg(short, long, default_value = "false")]
    no_voice: bool,
}

struct AppState {
    ply_name: String,
    ply_kills: u16,
    stream_handle: Arc<OutputStreamHandle>,
    args: Arc<Args>,
}
// Function to list available host devices
fn list_host_devices() {
    let host = cpal::default_host();
    let devices = host.output_devices().unwrap();
    info!("Available output devices:");
    for device in devices {
        let dev: rodio::Device = device.into();
        let dev_name: String = dev.name().unwrap();
        info!("{}", dev_name);
    }
}

// Get an `OutputStream` and `OutputStreamHandle` for a specific device
fn get_output_stream(device_name: &str) -> (OutputStream, OutputStreamHandle) {
    if device_name == "default" {
        return OutputStream::try_default().unwrap();
    }
    let host = cpal::default_host();
    let devices = host.output_devices().unwrap();
    for device in devices {
        let dev: rodio::Device = device.into();
        let dev_name: String = dev.name().unwrap();
        if dev_name == device_name {
            info!("Using device: {}", dev_name);
            return OutputStream::try_from_device(&dev).unwrap();
        }
    }
    // If the specified device is not found, fall back to the default
    warn!(
        "Specified device {} not found, using default output device.",
        device_name
    );
    OutputStream::try_default().unwrap()
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

    let current_hs_kills = state.round_killhs;

    let current_name = player.name.as_ref().unwrap();
    let original_name = &app_state.ply_name;

    if current_kills > original_kills && (current_name == original_name || original_name == "") {
        let sound_num = if current_kills > 5 { 5 } else { current_kills };

        let args = app_state.args.clone();

        let preset = args.preset.to_string();
        let volume = args.volume;

        let stream_handle = app_state.stream_handle.clone();
        thread::spawn(move || {
            let (controller, mixer) = rodio::dynamic_mixer::mixer::<i16>(2, 44100);
            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            let file: File;

            if preset == "crossfire" {
                file = File::open(format!("sounds/{}/common.wav", preset)).unwrap();

                if !args.no_voice {
                    if current_hs_kills == 1 && current_kills == 1 {
                        let file_hs =
                            File::open(format!("sounds/{}/headshot.wav", preset)).unwrap();
                        let source_hs = rodio::Decoder::new(BufReader::new(file_hs)).unwrap();
                        controller.add(source_hs);
                    } else if current_kills > 1 && current_kills < 6 {
                        let file_voice =
                            File::open(format!("sounds/{}/{}.wav", preset, sound_num)).unwrap();
                        let source_voice = rodio::Decoder::new(BufReader::new(file_voice)).unwrap();
                        controller.add(source_voice);
                    }
                }
            } else {
                file = File::open(format!("sounds/{}/{}.wav", preset, sound_num)).unwrap();
            }
            // let file = File::open(format!("sounds/{}/{}.wav", preset, sound_num)).unwrap();
            let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

            controller.add(source);

            sink.append(mixer);
            sink.set_volume(volume);
            sink.play();
            sink.sleep_until_end();
        });
        info!("player:{} kills:{}", current_name, current_kills);
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

    let app_state = Arc::new(Mutex::new(AppState {
        ply_name: "".to_string(),
        ply_kills: 0,
        stream_handle: Arc::new(output_stream.1),
        args: Arc::new(args),
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
