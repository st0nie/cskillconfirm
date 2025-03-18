use std::{fs::File, io::BufReader, sync::Arc};

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
        let preset = app_state.preset.clone();
        let sound_num_max;

        sound_num_max = preset.end;

        let sound_num = if current_kills > sound_num_max {
            sound_num_max
        } else {
            current_kills
        };

        let preset_name = args.preset.to_string();
        let volume = args.volume;

        let stream_handle = app_state.stream_handle.clone();
        tokio::spawn(async move {
            let (controller, mixer) = rodio::dynamic_mixer::mixer::<i16>(2, 44100);
            let sink = rodio::Sink::try_new(&stream_handle).unwrap();

            if preset.has_common {
                let file = File::open(format!("sounds/{}/common.wav", preset_name)).unwrap();
                let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                controller.add(source);
            }

            if preset.has_headshot && !args.no_voice && current_hs_kills == 1 && current_kills == 1
            {
                let file = if preset.has_variant && args.variant.is_some() {
                    File::open(format!(
                        "sounds/{}_v_{}/headshot.wav",
                        preset_name,
                        args.variant.as_ref().unwrap(),
                    ))
                    .unwrap()
                } else {
                    File::open(format!("sounds/{}/headshot.wav", preset_name)).unwrap()
                };
                let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                controller.add(source);
            }

            if preset.has_voice
                && !args.no_voice
                && (current_kills >= preset.start || !preset.has_headshot)
                && current_kills <= sound_num_max
                || !preset.has_common
            {
                let file = if preset.has_variant && args.variant.is_some() {
                    File::open(format!(
                        "sounds/{}_v_{}/{}.wav",
                        preset_name,
                        args.variant.as_ref().unwrap(),
                        sound_num
                    ))
                    .unwrap()
                } else {
                    File::open(format!("sounds/{}/{}.wav", preset_name, sound_num)).unwrap()
                };
                let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
                controller.add(source);
            }

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
