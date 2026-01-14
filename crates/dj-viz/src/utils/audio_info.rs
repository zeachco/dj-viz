//! Audio system diagnostics for Unix platforms.
//!
//! Provides detailed information about PulseAudio/PipeWire configuration
//! and available audio devices.

use std::process::Command;

/// Logs detailed audio system information on Unix systems
pub fn log_audio_info() {
    println!("\n=== Audio System Diagnostics ===\n");

    // Check which audio server is running
    println!("--- Audio Server ---");
    if is_running("pipewire") {
        println!("PipeWire: running");
        run_cmd("pipewire", &["--version"]);
    } else {
        println!("PipeWire: not running");
    }

    if is_running("pulseaudio") {
        println!("PulseAudio: running");
        run_cmd("pulseaudio", &["--version"]);
    } else {
        println!("PulseAudio: not running");
    }

    // List sinks (outputs)
    println!("\n--- Output Devices (Sinks) ---");
    run_cmd("pactl", &["list", "sinks", "short"]);

    // List sources (inputs)
    println!("\n--- Input Devices (Sources) ---");
    run_cmd("pactl", &["list", "sources", "short"]);

    // List what's currently playing
    println!("\n--- Active Playback (Sink Inputs) ---");
    run_cmd("pactl", &["list", "sink-inputs", "short"]);

    // Show default devices
    println!("\n--- Default Devices ---");
    run_cmd("pactl", &["get-default-sink"]);
    run_cmd("pactl", &["get-default-source"]);

    // Monitor sources available
    println!("\n--- Monitor Sources (for capturing output audio) ---");
    if let Ok(output) = Command::new("pactl")
        .args(["list", "sources", "short"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains(".monitor") {
                println!("  {}", line);
            }
        }
        if !stdout.contains(".monitor") {
            println!("  (none found - you may need to create a loopback)");
        }
    }

    println!("\n=== End Diagnostics ===\n");
}

fn is_running(process: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(process)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_cmd(cmd: &str, args: &[&str]) {
    match Command::new(cmd).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.is_empty() {
                for line in stdout.lines() {
                    println!("  {}", line);
                }
            }
            if !stderr.is_empty() && !output.status.success() {
                eprintln!("  (error: {})", stderr.trim());
            }
        }
        Err(_) => {
            println!("  ({} not found)", cmd);
        }
    }
}
