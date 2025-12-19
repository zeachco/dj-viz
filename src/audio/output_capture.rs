use std::process::Command;

const CAPTURE_SINK_NAME: &str = "dj-viz-capture";

/// Represents a PipeWire port with its type (input/output) and name
#[derive(Clone, Debug)]
pub struct PwPort {
    pub is_output: bool,
    pub name: String,
}

impl PwPort {
    pub fn display(&self) -> String {
        let tag = if self.is_output { "out" } else { "in" };
        format!("({}) {}", tag, self.name)
    }
}

/// Manages PipeWire stream discovery and search state
pub struct OutputCapture {
    pub search_active: bool,
    pub query: String,
    pub streams: Vec<PwPort>,
    pub filtered: Vec<PwPort>,
    pub selected_idx: usize,
    connected_ports: Vec<String>, // Track connected ports for cleanup
}

impl OutputCapture {
    pub fn new() -> Self {
        Self {
            search_active: false,
            query: String::new(),
            streams: Vec::new(),
            filtered: Vec::new(),
            selected_idx: 0,
            connected_ports: Vec::new(),
        }
    }

    /// Ensure the virtual capture sink exists
    fn ensure_capture_sink() -> bool {
        // Check if sink already exists
        if let Ok(output) = Command::new("pw-cli").args(["info", CAPTURE_SINK_NAME]).output() {
            if output.status.success() {
                return true;
            }
        }

        // Create the virtual sink using pactl (more reliable than pw-cli for this)
        let result = Command::new("pactl")
            .args([
                "load-module",
                "module-null-sink",
                &format!("sink_name={}", CAPTURE_SINK_NAME),
                &format!("sink_properties=device.description={}", CAPTURE_SINK_NAME),
            ])
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    println!("Created virtual sink: {}", CAPTURE_SINK_NAME);
                    true
                } else {
                    eprintln!("Failed to create virtual sink");
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to run pactl: {}", e);
                false
            }
        }
    }

    /// Start search mode: enumerate PipeWire ports and activate UI
    pub fn start_search(&mut self) {
        self.streams.clear();
        self.query.clear();
        self.selected_idx = 0;

        // Get output ports
        if let Ok(output) = Command::new("pw-link").arg("-o").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    let name = line.trim().to_string();
                    if !name.is_empty() {
                        self.streams.push(PwPort {
                            is_output: true,
                            name,
                        });
                    }
                }
            }
        }

        // Get input ports
        if let Ok(output) = Command::new("pw-link").arg("-i").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    let name = line.trim().to_string();
                    if !name.is_empty() {
                        self.streams.push(PwPort {
                            is_output: false,
                            name,
                        });
                    }
                }
            }
        }

        // Sort: outputs first, then inputs
        self.streams.sort_by(|a, b| {
            b.is_output.cmp(&a.is_output).then_with(|| a.name.cmp(&b.name))
        });

        self.filter();
        self.search_active = true;
    }

    /// Filter streams by current query (case-insensitive)
    pub fn filter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.filtered = self
            .streams
            .iter()
            .filter(|port| {
                query_lower.is_empty() || port.name.to_lowercase().contains(&query_lower)
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
        self.streams.clear();
        self.filtered.clear();
        self.selected_idx = 0;
    }

    /// Get the currently selected port
    pub fn selected(&self) -> Option<&PwPort> {
        self.filtered.get(self.selected_idx)
    }

    /// Select current item and connect to virtual capture sink
    /// Returns (selected_port_name, monitor_source_name, success)
    pub fn select_and_connect(&mut self) -> Option<(String, Option<String>, bool)> {
        let port = self.selected()?.clone();
        self.search_active = false;

        // Only connect if we selected an output port
        if !port.is_output {
            // For input ports, just return - can't route inputs
            return Some((port.name, None, false));
        }

        // Ensure capture sink exists
        if !Self::ensure_capture_sink() {
            return Some((port.name, None, false));
        }

        // Disconnect previous connections
        self.disconnect_all();

        // Find the sink's input ports
        let sink_inputs = Self::find_sink_inputs();
        if sink_inputs.is_empty() {
            eprintln!("Could not find capture sink inputs");
            return Some((port.name, None, false));
        }

        // Connect the selected output to the sink
        // Try to match channels (FL->FL, FR->FR, or just connect to first available)
        let success = self.connect_to_sink(&port.name, &sink_inputs);

        // The monitor source name for cpal
        let monitor_name = format!("{}.monitor", CAPTURE_SINK_NAME);

        Some((port.name, Some(monitor_name), success))
    }

    /// Find the input ports of our capture sink
    fn find_sink_inputs() -> Vec<String> {
        let mut inputs = Vec::new();

        if let Ok(output) = Command::new("pw-link").arg("-i").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    let name = line.trim();
                    if name.contains(CAPTURE_SINK_NAME) {
                        inputs.push(name.to_string());
                    }
                }
            }
        }

        inputs
    }

    /// Connect an output port to the capture sink
    fn connect_to_sink(&mut self, source: &str, sink_inputs: &[String]) -> bool {
        // Try to find matching channel (FL, FR, etc.)
        let source_channel = source.split(':').last().unwrap_or("");

        // Find a matching sink input or use the first one
        let target = sink_inputs.iter()
            .find(|s| s.ends_with(source_channel))
            .or_else(|| sink_inputs.first());

        let Some(target) = target else {
            return false;
        };

        let result = Command::new("pw-link")
            .arg(source)
            .arg(target)
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    self.connected_ports.push(source.to_string());
                    println!("Connected: {} -> {}", source, target);
                    true
                } else {
                    eprintln!("pw-link failed: {} -> {}", source, target);
                    false
                }
            }
            Err(e) => {
                eprintln!("Failed to run pw-link: {}", e);
                false
            }
        }
    }

    /// Disconnect all previously connected ports
    fn disconnect_all(&mut self) {
        let sink_inputs = Self::find_sink_inputs();

        for source in &self.connected_ports {
            for target in &sink_inputs {
                let _ = Command::new("pw-link")
                    .arg("-d")
                    .arg(source)
                    .arg(target)
                    .status();
            }
        }

        self.connected_ports.clear();
    }

    /// Get the monitor source name for the capture sink
    pub fn monitor_source_name() -> String {
        format!("{}.monitor", CAPTURE_SINK_NAME)
    }
}

impl Default for OutputCapture {
    fn default() -> Self {
        Self::new()
    }
}
