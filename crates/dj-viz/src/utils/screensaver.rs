//! Screensaver inhibitor for preventing screensaver during playback.

#[cfg(target_os = "linux")]
mod platform {
    use std::process::{Child, Command};

    /// Inhibits the screensaver while held. Uninhibits on drop.
    pub struct ScreensaverInhibitor {
        child: Option<Child>,
    }

    impl ScreensaverInhibitor {
        /// Create a new screensaver inhibitor.
        /// Uses systemd-inhibit to prevent screensaver/idle.
        pub fn new() -> Option<Self> {
            // Use systemd-inhibit which works on most modern Linux systems
            // It runs as long as the child process lives
            let child = Command::new("systemd-inhibit")
                .args([
                    "--what=idle",
                    "--who=dj-viz",
                    "--why=Audio visualization active",
                    "--mode=block",
                    "sleep",
                    "infinity",
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok();

            if child.is_some() {
                println!("Screensaver inhibited via systemd-inhibit");
            }

            Some(Self { child })
        }
    }

    impl Drop for ScreensaverInhibitor {
        fn drop(&mut self) {
            if let Some(ref mut child) = self.child {
                let _ = child.kill();
                let _ = child.wait();
                println!("Screensaver uninhibited");
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use windows::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, EXECUTION_STATE,
    };

    /// Inhibits the screensaver while held. Uninhibits on drop.
    pub struct ScreensaverInhibitor {
        _private: (),
    }

    impl ScreensaverInhibitor {
        /// Create a new screensaver inhibitor.
        /// Uses SetThreadExecutionState to prevent display sleep.
        pub fn new() -> Option<Self> {
            unsafe {
                let state: EXECUTION_STATE = ES_CONTINUOUS | ES_DISPLAY_REQUIRED;
                SetThreadExecutionState(state);
            }
            println!("Screensaver inhibited via SetThreadExecutionState");
            Some(Self { _private: () })
        }
    }

    impl Drop for ScreensaverInhibitor {
        fn drop(&mut self) {
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
            println!("Screensaver uninhibited");
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    /// Stub for unsupported platforms.
    pub struct ScreensaverInhibitor;

    impl ScreensaverInhibitor {
        pub fn new() -> Option<Self> {
            None
        }
    }
}

pub use platform::ScreensaverInhibitor;
