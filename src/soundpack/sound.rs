use std::{fs::File, io::BufReader, sync::Arc};

use anyhow::{Context, Result};
use rodio::mixer;
use tokio::task::JoinSet;
use tracing::{error, info};

use crate::soundpack::SoundContext;
use crate::util::state::AppState;

async fn add_file_to_mixer(file_name: &str, mixer: &mixer::Mixer) -> Result<()> {
    let file =
        File::open(file_name).with_context(|| format!("failed to open file: {file_name}"))?;
    let source = rodio::Decoder::new(BufReader::new(file))
        .with_context(|| format!("failed to decode file: {file_name:?}"))?;
    mixer.add(source);
    Ok(())
}

pub async fn play_audio(
    app_state_clone: Arc<AppState>,
    current_kills: u16,
    origin_hs_kills: u64,
    current_hs_kills: u64,
) -> Result<()> {
    let args = &app_state_clone.args;
    let preset = &app_state_clone.preset;
    let stream_handle = &app_state_clone.stream_handle;
    let volume = args.volume;

    let mixer = stream_handle.mixer().to_owned();

    // Create context for Lua script
    let ctx = SoundContext {
        kill_count: current_kills,
        is_headshot: current_hs_kills > origin_hs_kills,
        is_first_kill: current_kills == 1,
        preset_name: preset.preset_name.clone(),
        master_name: preset.master_name.clone(),
        variant: preset.variant.clone(),
    };

    // Get sound files from Lua script
    let sound_files = preset
        .lua_script
        .get_sounds(&ctx)
        .with_context(|| "failed to get sounds from Lua script".to_string())?;

    info!(
        "Lua returned {} sound files: {:?}",
        sound_files.len(),
        sound_files
    );

    let mut tasks = JoinSet::new();

    for file_path in sound_files {
        let mixer_clone = mixer.clone();
        tasks.spawn(async move { add_file_to_mixer(&file_path, &mixer_clone).await });
    }

    tokio::task::spawn_blocking(move || {
        let sink = rodio::Sink::connect_new(&mixer);
        sink.set_volume(volume);
        sink.play();
        sink.sleep_until_end();
    });

    let results = tasks.join_all().await;

    results.iter().for_each(|result| {
        if let Err(e) = result {
            error!("Failed to add file to mixer: {}", e);
        }
    });

    Ok(())
}
