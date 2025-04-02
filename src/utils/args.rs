use clap::Parser;
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// list all available audio devices
    #[arg(short, long, default_value = "false")]
    pub list_devices: bool,
    /// sound preset to use (available: "valorant", "crossfire", "bf1")
    #[arg(short, long, default_value = "crossfire")]
    pub preset: String,
    /// select output device
    #[arg(short, long, default_value = "default")]
    pub device: String,
    ///
    #[arg(short, long, default_value = "1.0")]
    pub volume: f32,
    /// disable voice for some presets
    #[arg(short, long, default_value = "false")]
    pub no_voice: bool,
    /// use variant of sound preset
    #[arg(long)]
    pub variant: Option<String>,
}
