use rodio::OutputStreamHandle;
use tokio::sync::RwLock;

use crate::soundpack::Preset;

use super::Args;

pub struct Mutable {
    pub steamid: String,
    pub ply_kills: u16,
    pub ply_hs_kills: u64,
}

pub struct AppState {
    pub mutable: RwLock<Mutable>,
    pub stream_handle: OutputStreamHandle,
    pub args: Args,
    pub preset: Preset,
}
