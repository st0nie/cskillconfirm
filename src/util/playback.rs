use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle};
use tracing::{self, info, warn};

// Function to list available host devices
pub fn list_host_devices() {
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
pub fn get_output_stream(device_name: &str) -> (OutputStream, OutputStreamHandle) {
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