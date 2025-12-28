//! Rhai scripting support for hot-reloadable visualizations.
//!
//! Allows writing visualizations in Rhai scripts that can be edited
//! and reloaded without restarting the application.

mod audio_api;
mod draw_api;

use crate::audio::AudioAnalysis;
use audio_api::update_audio_in_scope;
use draw_api::{register_draw_api, register_math_api, CommandQueue};
use nannou::prelude::*;
use rhai::{Dynamic, Engine, Scope, AST};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

/// Persistent variable store for scripts (survives hot-reload)
type VarStore = Rc<RefCell<HashMap<String, Dynamic>>>;

/// Register persistent variable functions on the engine
fn register_var_api(engine: &mut Engine, store: VarStore) {
    // get(name) - get a persistent variable, returns () if not set
    let s = store.clone();
    engine.register_fn("get", move |name: &str| -> Dynamic {
        s.borrow().get(name).cloned().unwrap_or(Dynamic::UNIT)
    });

    // set(name, value) - set a persistent variable
    let s = store.clone();
    engine.register_fn("set", move |name: &str, value: Dynamic| {
        s.borrow_mut().insert(name.to_string(), value);
    });

    // get_or(name, default) - get a persistent variable, or return default (doesn't store default)
    let s = store.clone();
    engine.register_fn("get_or", move |name: &str, default: Dynamic| -> Dynamic {
        s.borrow().get(name).cloned().unwrap_or(default)
    });

    // init(name, value) - set only if not already set, returns current value
    let s = store.clone();
    engine.register_fn("init", move |name: &str, value: Dynamic| -> Dynamic {
        let mut store = s.borrow_mut();
        if let Some(existing) = store.get(name) {
            existing.clone()
        } else {
            store.insert(name.to_string(), value.clone());
            value
        }
    });
}

/// Check interval for file modifications (in frames, ~0.5 sec at 60fps)
const RELOAD_CHECK_INTERVAL: u32 = 30;

/// Maximum operations per script execution (prevents infinite loops)
const MAX_OPERATIONS: u64 = 100_000;

/// Manages Rhai script discovery and cycling
pub struct ScriptManager {
    scripts_dir: PathBuf,
    script_paths: Vec<PathBuf>,
    current_index: Option<usize>,
    visualization: Option<ScriptedVisualization>,
}

impl ScriptManager {
    /// Create a new script manager that scans the given directory
    pub fn new(scripts_dir: PathBuf) -> Self {
        let mut manager = Self {
            scripts_dir,
            script_paths: Vec::new(),
            current_index: None,
            visualization: None,
        };
        manager.scan_scripts();
        manager
    }

    /// Scan the scripts directory for .rhai files
    pub fn scan_scripts(&mut self) {
        self.script_paths.clear();

        if let Ok(entries) = fs::read_dir(&self.scripts_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rhai") {
                    self.script_paths.push(path);
                }
            }
        }

        // Sort alphabetically for consistent ordering
        self.script_paths.sort();

        println!(
            "Found {} scripts in {:?}",
            self.script_paths.len(),
            self.scripts_dir
        );
        for path in &self.script_paths {
            println!("  - {:?}", path.file_name().unwrap_or_default());
        }
    }

    /// Cycle to the next script, returns the script name if successful
    pub fn cycle_next(&mut self) -> Option<String> {
        if self.script_paths.is_empty() {
            self.scan_scripts(); // Try rescanning
            if self.script_paths.is_empty() {
                return None;
            }
        }

        let next_index = match self.current_index {
            Some(idx) => (idx + 1) % self.script_paths.len(),
            None => 0,
        };

        self.load_script_at(next_index)
    }

    /// Load a script by index
    fn load_script_at(&mut self, index: usize) -> Option<String> {
        if index >= self.script_paths.len() {
            return None;
        }

        let path = &self.script_paths[index];
        match ScriptedVisualization::new(path.clone()) {
            Ok(viz) => {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                self.visualization = Some(viz);
                self.current_index = Some(index);
                println!("Loaded script: {}", name);
                Some(name)
            }
            Err(e) => {
                eprintln!("Failed to load script {:?}: {}", path, e);
                None
            }
        }
    }

    /// Check if a script is currently active
    pub fn is_active(&self) -> bool {
        self.visualization.is_some()
    }

    /// Deactivate scripted visualization (return to built-in)
    pub fn deactivate(&mut self) {
        self.visualization = None;
        self.current_index = None;
    }

    /// Update the current script visualization
    pub fn update(&mut self, analysis: &AudioAnalysis, bounds: Rect) {
        if let Some(ref mut viz) = self.visualization {
            viz.update(analysis, bounds);
        }
    }

    /// Draw the current script visualization
    pub fn draw(&self, draw: &Draw, bounds: Rect) {
        if let Some(ref viz) = self.visualization {
            viz.draw(draw, bounds);
        }
    }
}

/// A visualization powered by a Rhai script
pub struct ScriptedVisualization {
    engine: Engine,
    ast: Option<AST>,
    scope: Scope<'static>,
    commands: CommandQueue,
    vars: VarStore,
    script_path: PathBuf,
    last_modified: SystemTime,
    frame_counter: u32,
    check_counter: u32,
    last_error_frame: u32,
    bounds: Rect,
    /// True on first frame after script load/reload
    script_init: bool,
}

impl ScriptedVisualization {
    /// Create a new scripted visualization from a file path
    pub fn new(script_path: PathBuf) -> Result<Self, String> {
        let commands: CommandQueue = Rc::new(RefCell::new(Vec::new()));
        let vars: VarStore = Rc::new(RefCell::new(HashMap::new()));

        let mut engine = Engine::new();

        // Set operation limit to prevent infinite loops
        engine.set_max_operations(MAX_OPERATIONS);

        // Register APIs
        register_draw_api(&mut engine, commands.clone());
        register_math_api(&mut engine);
        register_var_api(&mut engine, vars.clone());

        // Get initial modification time
        let last_modified = fs::metadata(&script_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut viz = Self {
            engine,
            ast: None,
            scope: Scope::new(),
            commands,
            vars,
            script_path,
            last_modified,
            frame_counter: 0,
            check_counter: 0,
            last_error_frame: 0,
            bounds: Rect::from_w_h(640.0, 480.0),
            script_init: true,
        };

        // Load and compile the script
        viz.reload_script()?;

        Ok(viz)
    }

    /// Reload the script from disk
    fn reload_script(&mut self) -> Result<(), String> {
        let source =
            fs::read_to_string(&self.script_path).map_err(|e| format!("Read error: {}", e))?;

        let ast = self
            .engine
            .compile(&source)
            .map_err(|e| format!("Compile error: {}", e))?;

        self.ast = Some(ast);
        // Clear scope and persistent vars so script can reinitialize
        self.scope.clear();
        self.vars.borrow_mut().clear();
        // Signal first frame after reload
        self.script_init = true;
        println!("Script compiled: {:?}", self.script_path.file_name());

        Ok(())
    }

    /// Check if the script file has been modified and reload if needed
    fn check_reload(&mut self) {
        self.check_counter += 1;
        if self.check_counter < RELOAD_CHECK_INTERVAL {
            return;
        }
        self.check_counter = 0;

        if let Ok(metadata) = fs::metadata(&self.script_path) {
            if let Ok(modified) = metadata.modified() {
                if modified > self.last_modified {
                    self.last_modified = modified;
                    println!("Script modified, reloading...");

                    if let Err(e) = self.reload_script() {
                        eprintln!("Reload failed: {}", e);
                        // Keep using the previous AST
                    }
                }
            }
        }
    }

    /// Update the visualization with audio analysis
    pub fn update(&mut self, analysis: &AudioAnalysis, bounds: Rect) {
        self.frame_counter += 1;
        self.bounds = bounds;

        // Check for file changes
        self.check_reload();

        // Clear command queue
        self.commands.borrow_mut().clear();

        // Update audio data in scope (uses set_or_push to preserve user variables)
        update_audio_in_scope(
            &mut self.scope,
            analysis,
            bounds,
            self.frame_counter as i64,
        );

        // Update script_init flag (true on first frame after load/reload)
        self.scope.set_or_push("script_init", self.script_init);

        // Run the script
        if let Some(ref ast) = self.ast {
            if let Err(e) = self.engine.run_ast_with_scope(&mut self.scope, ast) {
                // Throttle error messages (once per second)
                if self.frame_counter - self.last_error_frame > 60 {
                    eprintln!("Script error: {}", e);
                    self.last_error_frame = self.frame_counter;
                }
            }
        }

        // Clear init flag after first successful frame
        self.script_init = false;
    }

    /// Draw the visualization
    pub fn draw(&self, draw: &Draw, _bounds: Rect) {
        // Clear background
        draw.background().color(BLACK);

        // Execute all queued draw commands
        for cmd in self.commands.borrow().iter() {
            cmd.execute(draw);
        }
    }
}
