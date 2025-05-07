use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub has_variant: bool,
    pub has_voice: bool,
    pub has_common: bool,
    pub has_headshot: bool,
    pub has_common_headshot: bool,
    pub start: u16,
    pub end: u16,
}

impl TryFrom<&str> for Preset {
    type Error = anyhow::Error;

    fn try_from(preset_name: &str) -> Result<Self> {
        let content = fs::read_to_string(format!("sounds/{}/info.json", preset_name))?;
        let preset: Preset = serde_json::from_str(&content)?;
        Ok(preset)
    }
}
