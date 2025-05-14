use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle};
use tracing::{self, info, warn};

// Function to list available host devices
pub fn list_host_devices() -> Result<()> {
    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .context("unable to get output devices")?;
    info!("Available output devices:");

    for device in devices {
        let dev: rodio::Device = device.into();
        let dev_name = dev.name().unwrap_or_default();
        info!("{}", dev_name);
    }

    Ok(())
}

// Get an `OutputStream` and `OutputStreamHandle` for a specific device
pub fn get_output_stream(device_name: &str) -> Result<(OutputStream, OutputStreamHandle)> {
    if device_name == "default" {
        return Ok(OutputStream::try_default()?);
    }
    let host = cpal::default_host();
    let devices = host.output_devices()?;
    for device in devices {
        let dev: rodio::Device = device.into();
        let dev_name: String = dev.name()?;
        if dev_name == device_name {
            info!("Using device: {}", dev_name);
            return Ok(OutputStream::try_from_device(&dev)?);
        }
    }
    // If the specified device is not found, fall back to the default
    warn!(
        "Specified device {} not found, using default output device.",
        device_name
    );
    Ok(OutputStream::try_default()?)
}
