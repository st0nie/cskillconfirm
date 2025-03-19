use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub has_variant: bool,
    pub has_voice: bool,
    pub has_headshot: bool,
    pub has_common: bool,
    pub start: u16,
    pub end: u16,
}

pub fn parse_from_name(preset_name: &str) -> Result<Preset> {
    let content = fs::read_to_string(format!("sounds/{}/info.json", preset_name))?;
    let preset: Preset = serde_json::from_str(&content)?;
    Ok(preset)
}
