use anyhow::Result;
use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use gsi_cs2::Body;
use thiserror::Error;
use tokio::signal;
use tracing::{error, info, warn};

use crate::soundpack::playback::play_audio;

use crate::AppState;

#[derive(Error, Debug)]
pub enum ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
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

    if let Some(whitelist) = &app_state.args.steamid {
        let steamid = player_data
            .as_ref()
            .unwrap()
            .steam_id
            .as_deref()
            .unwrap_or("");
        if steamid != whitelist {
            return Ok(StatusCode::OK);
        }
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
            "player: {}, kills: {}",
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
