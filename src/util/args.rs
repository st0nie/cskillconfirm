use clap::Parser;
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// select output device
    #[arg(short, long, default_value = "default")]
    pub device: String,
    /// list all available audio devices
    #[arg(short, long, default_value = "false")]
    pub list_devices: bool,
    /// disable voice for some presets
    #[arg(short, long, default_value = "false")]
    pub no_voice: bool,
    /// sound preset to use (available: "valorant", "crossfire", "bf1", "bf1_special")
    #[arg(short, long, default_value = "crossfire")]
    pub preset: String,
    /// play sound only for a specific steamid
    #[arg(long)]
    pub steamid: Option<String>,
    /// use variant of sound preset
    #[arg(long)]
    pub variant: Option<String>,
    ///
    #[arg(short, long, default_value = "1.0")]
    pub volume: f32,
    /// list all sound presets
    #[arg(short = 'L', long, default_value = "false")]
    pub list_presets: bool,
}
