//! Audio device search and selection UI.
//!
//! Manages the interactive fuzzy-search overlay for selecting audio input/output
//! devices at runtime.

use cpal::traits::{DeviceTrait, HostTrait};

/// Represents an audio device entry for search/selection
#[derive(Clone, Debug)]
pub struct AudioDeviceEntry {
    pub index: usize,
    pub is_input: bool,
    pub name: String,
}

impl AudioDeviceEntry {
    pub fn display(&self) -> String {
        let tag = if self.is_input { "in" } else { "out" };
        format!("{} {} - {}", self.index, tag, self.name)
    }
}

/// Manages audio device search and selection state
pub struct OutputCapture {
    pub search_active: bool,
    pub query: String,
    pub devices: Vec<AudioDeviceEntry>,
    pub filtered: Vec<AudioDeviceEntry>,
    pub selected_idx: usize,
}

impl OutputCapture {
    pub fn new() -> Self {
        Self {
            search_active: false,
            query: String::new(),
            devices: Vec::new(),
            filtered: Vec::new(),
            selected_idx: 0,
        }
    }

    /// Collect all audio devices using cpal (inputs first, then outputs)
    fn collect_devices() -> Vec<AudioDeviceEntry> {
        let host = cpal::default_host();
        let mut devices = Vec::new();
        let mut idx = 0;

        // Inputs first
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    devices.push(AudioDeviceEntry {
                        index: idx,
                        is_input: true,
                        name,
                    });
                    idx += 1;
                }
            }
        }

        // Then outputs
        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    devices.push(AudioDeviceEntry {
                        index: idx,
                        is_input: false,
                        name,
                    });
                    idx += 1;
                }
            }
        }

        devices
    }

    /// Start search mode: enumerate audio devices and activate UI
    pub fn start_search(&mut self) {
        self.devices = Self::collect_devices();
        self.query.clear();
        self.selected_idx = 0;
        self.filter();
        self.search_active = true;
    }

    /// Filter devices by current query (case-insensitive)
    pub fn filter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.filtered = self
            .devices
            .iter()
            .filter(|device| {
                query_lower.is_empty() || device.name.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();

        // Reset selection if out of bounds
        if self.selected_idx >= self.filtered.len() {
            self.selected_idx = 0;
        }
    }

    /// Append a character to the search query
    pub fn append_char(&mut self, c: char) {
        self.query.push(c);
        self.filter();
    }

    /// Delete last character from query
    pub fn backspace(&mut self) {
        self.query.pop();
        self.filter();
    }

    /// Move selection up (cycles)
    pub fn move_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        if self.selected_idx == 0 {
            self.selected_idx = self.filtered.len() - 1;
        } else {
            self.selected_idx -= 1;
        }
    }

    /// Move selection down (cycles)
    pub fn move_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected_idx = (self.selected_idx + 1) % self.filtered.len();
    }

    /// Cancel search mode
    pub fn cancel(&mut self) {
        self.search_active = false;
        self.query.clear();
        self.devices.clear();
        self.filtered.clear();
        self.selected_idx = 0;
    }

    /// Get the currently selected device
    pub fn selected(&self) -> Option<&AudioDeviceEntry> {
        self.filtered.get(self.selected_idx)
    }

    /// Select current item and return its device index
    /// Returns (device_name, device_index)
    pub fn select(&mut self) -> Option<(String, usize)> {
        let device = self.selected()?.clone();
        self.search_active = false;
        Some((device.name, device.index))
    }
}

impl Default for OutputCapture {
    fn default() -> Self {
        Self::new()
    }
}
