use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use num_complex::Complex;
use rustfft::FftPlanner;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::utils::Config;

pub const BUFFER_SIZE: usize = 1024;

// Transition detection constants
const HISTORY_SIZE: usize = 180; // ~3 seconds at 60fps
const TRANSITION_THRESHOLD: f32 = 0.3; // Minimum difference to trigger transition

/// Audio stream result with samples and transition detection
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub transition_detected: bool,
}

/// Tracks audio metrics over time to detect transitions
pub struct TransitionTracker {
    /// Rolling history of RMS values (volume)
    volume_history: Vec<f32>,
    /// Rolling history of high-frequency energy ratio
    freq_history: Vec<f32>,
    /// Index for circular buffer
    history_idx: usize,
    /// FFT planner for frequency analysis
    fft_planner: FftPlanner<f32>,
    /// Hann window for FFT
    fft_window: Vec<f32>,
    /// Whether we're currently in a "high" state (for edge detection)
    was_high_volume: bool,
    was_high_freq: bool,
}

impl TransitionTracker {
    fn new() -> Self {
        let fft_window: Vec<f32> = (0..BUFFER_SIZE)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / BUFFER_SIZE as f32).cos()))
            .collect();

        Self {
            volume_history: vec![0.0; HISTORY_SIZE],
            freq_history: vec![0.0; HISTORY_SIZE],
            history_idx: 0,
            fft_planner: FftPlanner::new(),
            fft_window,
            was_high_volume: false,
            was_high_freq: false,
        }
    }

    /// Update with new samples and return true if a transition was detected
    fn update(&mut self, samples: &[f32]) -> bool {
        // Calculate current RMS (volume)
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        // Calculate high-frequency energy ratio using FFT
        let freq_ratio = self.compute_high_freq_ratio(samples);

        // Store in history
        self.volume_history[self.history_idx] = rms;
        self.freq_history[self.history_idx] = freq_ratio;
        self.history_idx = (self.history_idx + 1) % HISTORY_SIZE;

        // Calculate recent average (last ~0.5 seconds = 30 frames)
        let recent_frames = 30;
        let recent_volume = self.recent_average(&self.volume_history, recent_frames);
        let recent_freq = self.recent_average(&self.freq_history, recent_frames);

        // Calculate longer-term average (full 3 seconds)
        let long_volume: f32 = self.volume_history.iter().sum::<f32>() / HISTORY_SIZE as f32;
        let long_freq: f32 = self.freq_history.iter().sum::<f32>() / HISTORY_SIZE as f32;

        // Detect transitions - significant difference between recent and long-term
        let volume_diff = (recent_volume - long_volume).abs();
        let freq_diff = (recent_freq - long_freq).abs();

        // Normalize differences relative to the signal level
        let norm_volume_diff = if long_volume > 0.01 { volume_diff / long_volume } else { volume_diff * 10.0 };
        let norm_freq_diff = freq_diff; // Already 0-1 range

        // Check for state transitions (edge detection)
        let is_high_volume = recent_volume > long_volume * 1.3;
        let is_high_freq = recent_freq > long_freq + 0.15;

        let volume_transition = is_high_volume != self.was_high_volume && norm_volume_diff > TRANSITION_THRESHOLD;
        let freq_transition = is_high_freq != self.was_high_freq && norm_freq_diff > TRANSITION_THRESHOLD;

        self.was_high_volume = is_high_volume;
        self.was_high_freq = is_high_freq;

        volume_transition || freq_transition
    }

    fn recent_average(&self, history: &[f32], frames: usize) -> f32 {
        let mut sum = 0.0;
        for i in 0..frames {
            let idx = (self.history_idx + HISTORY_SIZE - 1 - i) % HISTORY_SIZE;
            sum += history[idx];
        }
        sum / frames as f32
    }

    fn compute_high_freq_ratio(&mut self, samples: &[f32]) -> f32 {
        // Apply window and prepare FFT buffer
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .zip(self.fft_window.iter())
            .map(|(s, w)| Complex::new(s * w, 0.0))
            .collect();

        // Perform FFT
        let fft = self.fft_planner.plan_fft_forward(BUFFER_SIZE);
        fft.process(&mut buffer);

        // Calculate energy in frequency bands
        let num_bins = BUFFER_SIZE / 2;
        let mid_point = num_bins / 2;

        let low_energy: f32 = buffer[1..mid_point].iter().map(|c| c.norm_sqr()).sum();
        let high_energy: f32 = buffer[mid_point..num_bins].iter().map(|c| c.norm_sqr()).sum();

        let total = low_energy + high_energy;
        if total > 0.0 {
            high_energy / total
        } else {
            0.0
        }
    }
}

pub struct DeviceInfo {
    pub device: cpal::Device,
    pub name: String,
    pub is_input: bool,
}

pub struct SourcePipe {
    buffer: Arc<Mutex<Vec<f32>>>,
    devices: Vec<DeviceInfo>,
    current_device: usize,
    _stream: Option<Stream>,
    // Auto-gain normalization state
    smoothed_peak: f32,
    target_level: f32,
    // Transition detection
    transition_tracker: TransitionTracker,
}

impl SourcePipe {
    pub fn new() -> Self {
        let devices = Self::collect_devices();
        let buffer = Arc::new(Mutex::new(vec![0.0; BUFFER_SIZE]));

        // Try to load last used device from config
        let config = Config::load();
        let start_index = config
            .last_device
            .as_ref()
            .and_then(|name| {
                let is_input = config.last_device_is_input.unwrap_or(false);
                devices.iter().position(|d| d.name == *name && d.is_input == is_input)
            })
            .or_else(|| {
                // Prefer pipewire or pulse input devices (more reliable on Linux)
                devices.iter().position(|d| d.is_input && d.name == "pipewire")
            })
            .or_else(|| {
                devices.iter().position(|d| d.is_input && d.name == "pulse")
            })
            .or_else(|| {
                // Fall back to default output device for loopback capture
                let host = cpal::default_host();
                let default_output_name = host.default_output_device().and_then(|d| d.name().ok());
                default_output_name
                    .and_then(|name| devices.iter().position(|d| !d.is_input && d.name == name))
            })
            .unwrap_or(0);

        let stream = if !devices.is_empty() {
            Self::build_stream(&devices[start_index], Arc::clone(&buffer))
        } else {
            eprintln!("No audio devices found!");
            None
        };

        if let Some(ref _s) = stream {
            let info = &devices[start_index];
            let device_type = if info.is_input { "input" } else { "output" };
            println!("[{}] Selected: {} ({})", start_index, info.name, device_type);
        }

        Self {
            buffer,
            devices,
            current_device: start_index,
            _stream: stream,
            smoothed_peak: 0.1, // Start with a reasonable default
            target_level: 0.5,  // Target peak level for normalization
            transition_tracker: TransitionTracker::new(),
        }
    }

    pub fn list_devices() {
        let host = cpal::default_host();
        println!("\n=== Audio Devices ===");

        let mut idx = 0;
        if let Ok(inputs) = host.input_devices() {
            for device in inputs {
                if let Ok(name) = device.name() {
                    println!("  [{}] {} (input)", idx, name);
                    idx += 1;
                }
            }
        }
        if let Ok(outputs) = host.output_devices() {
            for device in outputs {
                if let Ok(name) = device.name() {
                    println!("  [{}] {} (output)", idx, name);
                    idx += 1;
                }
            }
        }
        println!("Use 0-9 (Shift for +10) to switch devices\n");
    }

    fn collect_devices() -> Vec<DeviceInfo> {
        let host = cpal::default_host();
        let mut devices = Vec::new();

        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo { device, name, is_input: true });
                }
            }
        }

        if let Ok(output_devices) = host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    devices.push(DeviceInfo { device, name, is_input: false });
                }
            }
        }

        devices
    }

    fn device_timeout() -> Duration {
        Duration::from_secs(Config::load().device_timeout_secs())
    }

    /// Get device config with timeout (the config call often hangs on bad devices)
    fn get_config_with_timeout(device: &Device, is_input: bool) -> Option<StreamConfig> {
        let timeout = Self::device_timeout();
        let device_clone = device.clone();

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let config = if is_input {
                device_clone.default_input_config()
            } else {
                device_clone.default_output_config()
            };
            let _ = tx.send(config);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(config)) => Some(config.into()),
            Ok(Err(e)) => {
                eprintln!("  Failed to get config: {}", e);
                None
            }
            Err(_) => {
                eprintln!("  Device config timed out after {:?}", timeout);
                None
            }
        }
    }

    fn build_stream(device_info: &DeviceInfo, audio_buffer: Arc<Mutex<Vec<f32>>>) -> Option<Stream> {
        let stream_config = Self::get_config_with_timeout(&device_info.device, device_info.is_input)?;
        let channels = stream_config.channels as usize;

        let err_fn = |err| eprintln!("Audio stream error: {}", err);

        let stream = device_info.device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = audio_buffer.lock().unwrap();
                for chunk in data.chunks(channels) {
                    let sample: f32 = chunk.iter().sum::<f32>() / channels as f32;
                    buffer.remove(0);
                    buffer.push(sample);
                }
            },
            err_fn,
            None,
        );

        match stream {
            Ok(s) => {
                if let Err(e) = s.play() {
                    eprintln!("  Failed to play stream: {}", e);
                    return None;
                }
                Some(s)
            }
            Err(e) => {
                eprintln!("  Failed to build stream: {}", e);
                None
            }
        }
    }

    /// Attempts to select a device.
    /// Returns Some((device_name, success)) if a switch was attempted, None if index invalid/same.
    pub fn select_device(&mut self, index: usize) -> Option<(String, bool)> {
        if index >= self.devices.len() {
            return None;
        }
        if index == self.current_device {
            let info = &self.devices[index];
            return Some((info.name.clone(), true));
        }

        let info = &self.devices[index];
        let device_type = if info.is_input { "input" } else { "output" };
        let device_name = info.name.clone();
        let is_input = info.is_input;
        println!("[{}] Selecting: {} ({})", index, device_name, device_type);

        // Clear the buffer
        {
            let mut buf = self.buffer.lock().unwrap();
            buf.iter_mut().for_each(|x| *x = 0.0);
        }

        if let Some(stream) = Self::build_stream(info, Arc::clone(&self.buffer)) {
            println!("  -> OK");
            self._stream = Some(stream);
            self.current_device = index;

            // Save to config
            let mut config = Config::load();
            config.set_device(&device_name, is_input);

            Some((device_name, true))
        } else {
            println!("  -> FAILED");
            Some((device_name, false))
        }
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Select a device by name (partial match, case-insensitive)
    /// Matches against "name (input)" or "name (output)" format
    /// Returns Some((device_name, success)) if found and switched
    pub fn select_device_by_name(&mut self, name: &str) -> Option<(String, bool)> {
        let name_lower = name.to_lowercase();
        let index = self.devices.iter().position(|d| {
            let device_type = if d.is_input { "input" } else { "output" };
            let full_name = format!("{} ({})", d.name, device_type).to_lowercase();
            full_name.contains(&name_lower) || d.name.to_lowercase().contains(&name_lower)
        })?;
        self.select_device(index)
    }

    /// Refresh the device list and return new count
    pub fn refresh_devices(&mut self) {
        self.devices = Self::collect_devices();
    }

    pub fn stream(&mut self) -> AudioFrame {
        let buffer = self.buffer.lock().unwrap().clone();

        // Calculate current peak level (absolute max)
        let current_peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Smooth the peak tracking (slow attack, slower release for stability)
        if current_peak > self.smoothed_peak {
            // Fast attack when signal gets louder
            self.smoothed_peak = self.smoothed_peak * 0.8 + current_peak * 0.2;
        } else {
            // Slow release when signal gets quieter
            self.smoothed_peak = self.smoothed_peak * 0.995 + current_peak * 0.005;
        }

        // Prevent division by zero and limit gain range
        let safe_peak = self.smoothed_peak.max(0.001);
        let gain = (self.target_level / safe_peak).clamp(0.5, 10.0);

        // Apply gain normalization
        let samples: Vec<f32> = buffer.iter().map(|s| (s * gain).clamp(-1.0, 1.0)).collect();

        // Detect transitions based on normalized samples
        let transition_detected = self.transition_tracker.update(&samples);

        AudioFrame {
            samples,
            transition_detected,
        }
    }
}
