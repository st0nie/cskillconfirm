use std::{fs::File, io::BufReader, sync::Arc};

use anyhow::{Context, Result};
use tokio::task::JoinSet;
use tracing::error;

use crate::AppState;

async fn add_file_to_mixer(
    file_name: &str,
    controller: &rodio::dynamic_mixer::DynamicMixerController<i16>,
) -> Result<()> {
    let file =
        File::open(file_name).with_context(|| format!("failed to open file: {}", file_name))?;
    let source = rodio::Decoder::new(BufReader::new(file))
        .with_context(|| format!("failed to decode file: {:?}", file_name))?;
    controller.add(source);
    Ok(())
}

pub async fn play_audio(
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
    let preset_name = Arc::new(args.preset.to_string());
    let volume = args.volume;

    let (controller, mixer) = rodio::dynamic_mixer::mixer::<i16>(2, 44100);
    let sink = rodio::Sink::try_new(stream_handle)?;

    let mut tasks = JoinSet::new();

    let preset_name_clone = preset_name.clone();
    let controller_clone = controller.clone();
    if preset.has_common_headshot && current_hs_kills > origin_hs_kills {
        tasks.spawn(async move {
            add_file_to_mixer(
                &format!("sounds/{}/common_headshot.wav", preset_name_clone),
                &controller_clone,
            )
            .await
        });
    } else if preset.has_common {
        tasks.spawn(async move {
            add_file_to_mixer(
                &format!("sounds/{}/common.wav", preset_name_clone),
                &controller_clone,
            )
            .await
        });
    }

    let controller_clone = controller.clone();
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
        tasks.spawn(async move { add_file_to_mixer(&file_path, &controller_clone).await });
    }

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
        tasks.spawn(async move { add_file_to_mixer(&file_path, &controller).await });
    }

    let results = tasks.join_all().await;

    sink.append(mixer);
    sink.set_volume(volume);
    sink.play();

    results.iter().for_each(|result| {
        if let Err(e) = result {
            error!("Failed to add file to mixer: {}", e);
        }
    });

    sink.sleep_until_end();
    Ok(())
}
