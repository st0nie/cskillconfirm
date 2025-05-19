use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

pub fn list() -> Result<()> {
    let path = fs::read_dir("sounds")?;

    let mut mp: HashMap<String, Vec<String>> = HashMap::new();

    for path in path {
        let path = path?;
        let file_name = path.file_name().to_string_lossy().to_string();

        let preset: Vec<&str> = file_name.split("_v_").collect();

        let preset_name = preset[0].to_string();
        let variant = preset.get(1);

        if mp.contains_key(preset_name.as_str()) == false {
            mp.insert(preset_name.clone(), vec![]);
        }

        if let Some(variant) = variant {
            mp.get_mut(preset_name.as_str())
                .unwrap()
                .push(variant.to_string());
        }
    }

    let mut keys: Vec<&String> = mp.keys().collect();
    keys.sort();

    for key in keys {
        let variants = mp.get(key).unwrap();
        if variants.is_empty() {
            println!("{}", key);
            continue;
        }

        println!("{}: [{}]", key, variants.join(", "));
    }

    Ok(())
}
