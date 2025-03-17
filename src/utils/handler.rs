use std::{fs::File, io::BufReader, sync::Arc, thread};

use axum::{extract::State, Json};
use gsi_cs2::Body;
use tokio::{signal, sync::Mutex};
use tracing::info;

use crate::AppState;

pub async fn update(State(app_state): State<Arc<Mutex<AppState>>>, data: Json<Body>) {
    let map = data.map.as_ref();
    if let None = map {
        return;
    }

    let player_data = data.player.as_ref();
    if let None = player_data {
        return;
    }

    let ply = player_data.unwrap();
    let ply_state = ply.state.as_ref().unwrap();

    let mut app_state = app_state.lock().await;

    let current_kills = ply_state.round_kills;
    let original_kills = app_state.ply_kills;

    let current_hs_kills = ply_state.round_killhs;

    let current_name = if let Some(name) = &ply.name { name } else { "" };
    let original_name = &app_state.ply_name;

    if current_kills > original_kills && (current_name == original_name || original_name == "") {
        let args = app_state.args.clone();
        let sound_num_max;

        sound_num_max = match args.preset.as_str() {
            "crossfire" => 8,
            _ => 5,
        };

        let sound_num = if current_kills > sound_num_max {
            sound_num_max
        } else {
            current_kills
        };

        let preset = args.preset.to_string();
        let volume = args.volume;

        let stream_handle = app_state.stream_handle.clone();
        thread::spawn(move || {
            let (controller, mixer) = rodio::dynamic_mixer::mixer::<i16>(2, 44100);
            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            let file: File;

            if preset == "crossfire" {
                file = File::open(format!("sounds/{}/common.wav", preset)).unwrap();

                let headshot_path: String;
                let voice_path: String;

                if let Some(variant) = &args.variant {
                    headshot_path = format!("sounds/{}_v_{}/headshot.wav", preset, variant);
                    voice_path = format!("sounds/{}_v_{}/{}.wav", preset, variant, sound_num);
                } else {
                    headshot_path = format!("sounds/{}/headshot.wav", preset);
                    voice_path = format!("sounds/{}/{}.wav", preset, sound_num);
                }

                if !args.no_voice {
                    if current_hs_kills == 1 && current_kills == 1 {
                        let file_hs = File::open(headshot_path).unwrap();
                        let source_hs = rodio::Decoder::new(BufReader::new(file_hs)).unwrap();
                        controller.add(source_hs);
                    } else if current_kills > 1 && current_kills <= 8 {
                        let file_voice = File::open(voice_path).unwrap();
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

pub async fn shutdown_signal() {
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
