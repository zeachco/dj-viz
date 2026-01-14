//! Plugin loading and hot-reload system
//!
//! Loads visualization plugins from dynamic libraries (.dylib/.so/.dll)
//! and watches for file changes to enable hot-reload.

use dj_viz_api::{PluginMetadata, Visualization_TO, ABI_VERSION};
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Errors that can occur during plugin loading
#[derive(Debug)]
pub enum PluginError {
    NotFound,
    InvalidPath,
    LoadFailed(String),
    MissingSymbol,
    IncompatibleABI { found: u32, expected: u32 },
    IoError(std::io::Error),
}

impl From<std::io::Error> for PluginError {
    fn from(e: std::io::Error) -> Self {
        PluginError::IoError(e)
    }
}

impl From<libloading::Error> for PluginError {
    fn from(e: libloading::Error) -> Self {
        PluginError::LoadFailed(e.to_string())
    }
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::NotFound => write!(f, "Plugin not found"),
            PluginError::InvalidPath => write!(f, "Invalid plugin path"),
            PluginError::LoadFailed(e) => write!(f, "Failed to load plugin: {}", e),
            PluginError::MissingSymbol => write!(f, "Required symbol not found in plugin"),
            PluginError::IncompatibleABI { found, expected } => {
                write!(f, "Incompatible ABI version: found {}, expected {}", found, expected)
            }
            PluginError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for PluginError {}

/// A loaded plugin instance
pub struct LoadedPlugin {
    pub metadata: PluginMetadata,
    pub instance: Visualization_TO<'static, std::boxed::Box<()>>,
    pub last_modified: SystemTime,
    pub library_path: PathBuf,
}

/// Manages plugin discovery, loading, and hot-reload
pub struct PluginLoader {
    plugins: HashMap<String, LoadedPlugin>,
    libraries: HashMap<String, Library>,
    plugins_dir: PathBuf,
    reload_check_counter: u32,
}

impl PluginLoader {
    const RELOAD_CHECK_INTERVAL: u32 = 30; // Check every 30 frames (~0.5s at 60fps)

    /// Create a new plugin loader that scans the given directory
    pub fn new(plugins_dir: PathBuf) -> Result<Self, PluginError> {
        let mut loader = Self {
            plugins: HashMap::new(),
            libraries: HashMap::new(),
            plugins_dir,
            reload_check_counter: 0,
        };

        loader.scan_plugins()?;

        Ok(loader)
    }

    /// Get platform-specific library extension
    fn lib_extension() -> &'static str {
        if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        }
    }

    /// Scan the plugins directory for compatible libraries
    fn scan_plugins(&mut self) -> Result<(), PluginError> {
        if !self.plugins_dir.exists() {
            println!("Plugins directory {:?} does not exist yet", self.plugins_dir);
            return Ok(());
        }

        let extension = Self::lib_extension();

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |e| e == extension) {
                if let Err(e) = self.load_plugin(&path) {
                    eprintln!("Failed to load plugin {:?}: {}", path, e);
                }
            }
        }

        println!("Loaded {} plugins from {:?}", self.plugins.len(), self.plugins_dir);

        Ok(())
    }

    /// Load a single plugin from a file path
    pub fn load_plugin(&mut self, path: &Path) -> Result<(), PluginError> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or(PluginError::InvalidPath)?
            .to_string();

        unsafe {
            let lib = Library::new(path)?;

            // Check ABI version compatibility
            let abi_version: Symbol<extern "C" fn() -> u32> =
                lib.get(b"abi_version").map_err(|_| PluginError::MissingSymbol)?;
            let found_version = abi_version();
            if found_version != ABI_VERSION {
                return Err(PluginError::IncompatibleABI {
                    found: found_version,
                    expected: ABI_VERSION,
                });
            }

            // Get metadata
            let metadata_fn: Symbol<extern "C" fn() -> PluginMetadata> =
                lib.get(b"plugin_metadata").map_err(|_| PluginError::MissingSymbol)?;
            let metadata = metadata_fn();

            // Create instance
            let create_fn: Symbol<extern "C" fn() -> Visualization_TO<'static, std::boxed::Box<()>>> =
                lib.get(b"create_visualization").map_err(|_| PluginError::MissingSymbol)?;
            let instance = create_fn();

            let last_modified = path.metadata()?.modified()?;

            println!("Loaded plugin: {} v{}", metadata.name, metadata.version);

            // Store library and plugin
            self.libraries.insert(name.clone(), lib);
            self.plugins.insert(
                name,
                LoadedPlugin {
                    metadata,
                    instance,
                    last_modified,
                    library_path: path.to_path_buf(),
                },
            );
        }

        Ok(())
    }

    /// Check for modified plugins and reload them
    ///
    /// This should be called once per frame. It rate-limits checks to
    /// avoid excessive file system operations.
    pub fn check_reload(&mut self) -> Vec<String> {
        self.reload_check_counter += 1;
        if self.reload_check_counter < Self::RELOAD_CHECK_INTERVAL {
            return Vec::new();
        }
        self.reload_check_counter = 0;

        let mut reloaded = Vec::new();

        // Collect names to reload (can't modify HashMap while iterating)
        let to_reload: Vec<String> = self
            .plugins
            .iter()
            .filter_map(|(name, plugin)| {
                if let Ok(metadata) = plugin.library_path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if modified > plugin.last_modified {
                            return Some(name.clone());
                        }
                    }
                }
                None
            })
            .collect();

        // Reload each modified plugin
        for name in to_reload {
            if let Err(e) = self.reload_plugin(&name) {
                eprintln!("Reload failed for {}: {}", name, e);
            } else {
                reloaded.push(name);
            }
        }

        reloaded
    }

    /// Reload a specific plugin by name
    fn reload_plugin(&mut self, name: &str) -> Result<(), PluginError> {
        let plugin = self.plugins.get(name).ok_or(PluginError::NotFound)?;
        let path = plugin.library_path.clone();

        // Unload old version
        self.plugins.remove(name);
        self.libraries.remove(name);

        // Brief delay to ensure file is fully written
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Load new version
        self.load_plugin(&path)?;
        println!("🔁 Reloaded plugin: {}", name);

        Ok(())
    }

    /// Get all loaded plugins
    pub fn get_all_plugins(&self) -> Vec<&LoadedPlugin> {
        self.plugins.values().collect()
    }

    /// Get all loaded plugins (mutable)
    pub fn get_all_plugins_mut(&mut self) -> Vec<&mut LoadedPlugin> {
        self.plugins.values_mut().collect()
    }

    /// Get a specific plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    /// Get a specific plugin by name (mutable)
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut LoadedPlugin> {
        self.plugins.get_mut(name)
    }

    /// Get number of loaded plugins
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}
