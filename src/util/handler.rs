use anyhow::{Context, Result};
use std::{fs::File, io::BufReader, sync::Arc};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use gsi_cs2::Body;
use thiserror::Error;
use tokio::signal;
use tracing::{error, info, warn};

use crate::AppState;

#[derive(Error, Debug)]
pub enum ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn play_audio(
    app_state_clone: Arc<AppState>,
    sound_num: u16,
    current_kills: u16,
    origin_hs_kills: u64,
    current_hs_kills: u64,
    sound_num_max: u16,
) -> Result<()> {
    let args = &app_state_clone.args;
    let preset = &app_state_clone.preset;
    let stream_handle = &app_state_clone.stream_handle;
    let preset_name = args.preset.to_string();
    let volume = args.volume;

    let (controller, mixer) = rodio::dynamic_mixer::mixer::<i16>(2, 44100);
    let sink = rodio::Sink::try_new(stream_handle)?;

    let add_file_to_mixer = async |file_name: &str| -> Result<()> {
        let file =
            File::open(file_name).with_context(|| format!("failed to open file: {}", file_name))?;
        let source = rodio::Decoder::new(BufReader::new(file))
            .with_context(|| format!("failed to decode file: {:?}", file_name))?;
        controller.add(source);
        anyhow::Ok(())
    };

    let play_common = async  {
        if preset.has_common_headshot && current_hs_kills > origin_hs_kills {
            add_file_to_mixer(&format!("sounds/{}/common_headshot.wav", preset_name)).await?
        } else if preset.has_common {
            add_file_to_mixer(&format!("sounds/{}/common.wav", preset_name)).await?
        }
        anyhow::Ok(())
    };

    let play_headshot = async {
        if preset.has_headshot && !args.no_voice && current_hs_kills == 1 && current_kills == 1 {
            let file_path = if preset.has_variant && args.variant.is_some() {
                format!(
                    "sounds/{}_v_{}/headshot.wav",
                    preset_name,
                    args.variant.as_ref().unwrap()
                )
            } else {
                format!("sounds/{}/headshot.wav", preset_name)
            };
            add_file_to_mixer(&file_path).await?
        }
        anyhow::Ok(())
    };

    let play_voice = async  {
        if preset.has_voice
            && !args.no_voice
            && (current_kills >= preset.start || !preset.has_headshot)
            && current_kills <= sound_num_max
            || !preset.has_common
        {
            let file_path = if preset.has_variant && args.variant.is_some() {
                format!(
                    "sounds/{}_v_{}/{}.wav",
                    preset_name,
                    args.variant.as_ref().unwrap(),
                    sound_num
                )
            } else {
                format!("sounds/{}/{}.wav", preset_name, sound_num)
            };
            add_file_to_mixer(&file_path).await?;
        }

        anyhow::Ok(())
    };

    let results = vec![
        play_common.await,
        play_headshot.await,
        play_voice.await,
    ];

    sink.append(mixer);
    sink.set_volume(volume);
    sink.play();

    results.iter().for_each(|result| {
        if let Err(e) = result {
            error!("Error playing sound: {}", e);
        }
    });

    sink.sleep_until_end();
    Ok(())
}

pub async fn update(
    State(app_state): State<Arc<AppState>>,
    data: Json<Body>,
) -> Result<StatusCode, ApiError> {
    let map = data.map.as_ref();
    let player_data = data.player.as_ref();

    if map.is_none() || player_data.is_none() {
        warn!("map or player data is missing");
        return Ok(StatusCode::OK);
    }

    let ply = player_data.unwrap();
    let ply_state = ply.state.as_ref().unwrap();

    let binding = app_state.mutable.read().await;
    let current_kills = ply_state.round_kills;
    let original_kills = binding.ply_kills;

    let current_hs_kills = ply_state.round_killhs;
    let origin_hs_kills = binding.ply_hs_kills;

    let original_steamid = binding.steamid.clone();
    drop(binding);

    let steamid = ply.steam_id.as_deref().unwrap_or("");

    if current_kills > original_kills && (steamid == original_steamid || original_steamid == "") {
        let app_state_clone = app_state.clone();
        // Note: args access moved inside tokio::spawn
        let sound_num_max;

        sound_num_max = app_state.preset.end;

        let sound_num = if current_kills > sound_num_max {
            sound_num_max
        } else {
            current_kills
        };

        tokio::spawn(async move {
            let result = play_audio(
                app_state_clone,
                sound_num,
                current_kills,
                origin_hs_kills,
                current_hs_kills,
                sound_num_max,
            )
            .await;

            if result.is_err() {
                error!("Failed to play audio: {}", result.unwrap_err());
            }
        });
        info!(
            "player:{} kills:{}",
            ply.name.as_deref().unwrap_or(""),
            current_kills
        );
    }

    let mut binding = app_state.mutable.write().await;

    binding.ply_kills = current_kills;
    binding.ply_hs_kills = current_hs_kills;
    binding.steamid = steamid.to_string();

    Ok(StatusCode::OK)
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
