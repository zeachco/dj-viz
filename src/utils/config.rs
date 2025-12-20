//! Configuration file management.
//!
//! Handles loading and saving user preferences to `~/.dj-viz.toml`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_DEVICE_TIMEOUT_SECS: u64 = 3;

const CONFIG_TEMPLATE: &str = r#"# dj-viz configuration file

# Timeout in seconds when switching audio devices (default: 3)
# device_timeout_secs = 3

# Last selected audio device (auto-saved)
# last_device = "Device Name"
# last_device_is_input = false

# Last selected PipeWire stream target (auto-saved)
# pw_link_target = "Spotify:output_FL"
"#;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub last_device: Option<String>,
    pub last_device_is_input: Option<bool>,
    pub device_timeout_secs: Option<u64>,
    pub pw_link_target: Option<String>,
}

impl Config {
    fn path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".dj-viz.toml"))
    }

    pub fn load() -> Self {
        let path = match Self::path() {
            Some(p) => p,
            None => return Self::default(),
        };

        // Create template file if it doesn't exist
        if !path.exists() {
            let _ = fs::write(&path, CONFIG_TEMPLATE);
            println!("Created config template at {:?}", path);
        }

        fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn device_timeout_secs(&self) -> u64 {
        self.device_timeout_secs
            .unwrap_or(DEFAULT_DEVICE_TIMEOUT_SECS)
    }

    pub fn save(&self) {
        if let Some(path) = Self::path() {
            if let Ok(content) = toml::to_string(self) {
                let _ = fs::write(&path, &content);
                println!("Config saved to {:?}", path);
            }
        }
    }

    pub fn set_device(&mut self, name: &str, is_input: bool) {
        self.last_device = Some(name.to_string());
        self.last_device_is_input = Some(is_input);
        self.save();
    }
}
