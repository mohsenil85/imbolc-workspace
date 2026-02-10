use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::backend::{AudioBackend, RawArg, ScBackend};
use super::{AudioEngine, ServerStatus, GROUP_SOURCES, GROUP_PROCESSING, GROUP_OUTPUT, GROUP_BUS_PROCESSING, GROUP_RECORD, GROUP_SAFETY};
use crate::osc_client::{AudioMonitor, OscClient};
use regex::Regex;

/// Result of spawning scsynth in a background thread.
pub(crate) struct ServerSpawnResult {
    pub child: Child,
    pub use_pw_jack: bool,
}

/// Spawn scsynth in the current thread (meant to be called from a background thread).
/// Handles device resolution, argument building, and process spawning.
/// Returns the Child process on success.
fn spawn_scsynth(
    input_device: Option<String>,
    output_device: Option<String>,
    buffer_size: u32,
) -> Result<ServerSpawnResult, String> {
    let scsynth_paths = [
        "scsynth",
        "/Applications/SuperCollider.app/Contents/Resources/scsynth",
        "/usr/local/bin/scsynth",
        "/usr/bin/scsynth",
    ];

    // Build args: base port + buffer size + optional device flags
    let mut args: Vec<String> = vec![
        "-u".to_string(), "57110".to_string(),
        "-Z".to_string(), buffer_size.to_string(),
    ];

    // Resolve "System Default" to actual device names so we always
    // pass -H to scsynth. Without -H, scsynth probes all devices
    // and can crash on incompatible ones (e.g. iPhone continuity mic).
    let (default_output, default_input) = crate::devices::default_device_names();
    let resolved_input = input_device.or(default_input);
    let resolved_output = output_device.or(default_output);

    match (resolved_input.as_deref(), resolved_output.as_deref()) {
        (Some(inp), Some(out)) if inp != out => {
            args.push("-H".to_string());
            args.push(inp.to_string());
            args.push(out.to_string());
        }
        (Some(dev), None) | (None, Some(dev)) => {
            args.push("-H".to_string());
            args.push(dev.to_string());
        }
        (Some(dev), Some(_)) => {
            // Same device for both
            args.push("-H".to_string());
            args.push(dev.to_string());
        }
        (None, None) => {}
    }

    // Redirect scsynth output to a log file for crash diagnostics
    let log_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("imbolc")
        .join("scsynth.log");
    let _ = fs::create_dir_all(log_path.parent().unwrap());
    let stdout_file = fs::File::create(&log_path).ok();
    let stderr_file = stdout_file.as_ref().and_then(|f| f.try_clone().ok());

    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // On Linux, try launching scsynth via pw-jack so it routes through
    // PipeWire's JACK emulation instead of requiring a standalone JACK daemon.
    let use_pw_jack = cfg!(target_os = "linux")
        && Command::new("pw-jack")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok();

    for path in &scsynth_paths {
        let mut cmd = if use_pw_jack {
            let mut c = Command::new("pw-jack");
            c.arg(path);
            c
        } else {
            Command::new(path)
        };
        match cmd
            .args(&arg_refs)
            .stdout(stdout_file.as_ref()
                .and_then(|f| f.try_clone().ok())
                .map(Stdio::from)
                .unwrap_or_else(Stdio::null))
            .stderr(stderr_file.as_ref()
                .and_then(|f| f.try_clone().ok())
                .map(Stdio::from)
                .unwrap_or_else(Stdio::null))
            .spawn()
        {
            Ok(child) => {
                return Ok(ServerSpawnResult { child, use_pw_jack });
            }
            Err(_) => continue,
        }
    }

    Err("Could not find scsynth. Install SuperCollider.".to_string())
}

impl AudioEngine {
    #[allow(dead_code)]
    pub fn start_server(&mut self) -> Result<(), String> {
        self.start_server_with_devices(None, None, 512, 44100)
    }

    /// Synchronous server start — used by the `StartServer` command which has a reply channel.
    pub fn start_server_with_devices(
        &mut self,
        input_device: Option<&str>,
        output_device: Option<&str>,
        buffer_size: u32,
        sample_rate: u32,
    ) -> Result<(), String> {
        if self.scsynth_process.is_some() {
            return Err("Server already running".to_string());
        }

        self.server_status = ServerStatus::Starting;
        let _ = sample_rate;

        let result = spawn_scsynth(
            input_device.map(|s| s.to_string()),
            output_device.map(|s| s.to_string()),
            buffer_size,
        )?;

        self.scsynth_process = Some(result.child);
        self.server_status = ServerStatus::Running;

        if result.use_pw_jack {
            Self::connect_jack_ports();
        }

        Ok(())
    }

    /// Async server start — spawns scsynth in a background thread and returns a
    /// receiver that delivers the result. The audio thread polls this via
    /// `pending_server_start` in `poll_engine()`.
    pub(crate) fn start_server_async(
        &mut self,
        input_device: Option<String>,
        output_device: Option<String>,
        buffer_size: u32,
    ) -> Result<mpsc::Receiver<Result<ServerSpawnResult, String>>, String> {
        if self.scsynth_process.is_some() {
            return Err("Server already running".to_string());
        }

        self.server_status = ServerStatus::Starting;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = spawn_scsynth(input_device, output_device, buffer_size);
            let _ = tx.send(result);
        });

        Ok(rx)
    }

    /// Install a successfully spawned scsynth child into the engine.
    pub(crate) fn install_server_child(&mut self, result: ServerSpawnResult) {
        self.scsynth_process = Some(result.child);
        self.server_status = ServerStatus::Running;

        if result.use_pw_jack {
            Self::connect_jack_ports();
        }
    }

    /// Check if the scsynth child process has exited unexpectedly.
    /// Returns `Some(message)` if it died, `None` if healthy.
    pub fn check_server_health(&mut self) -> Option<String> {
        if let Some(ref mut child) = self.scsynth_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let live_before = self.node_registry.live_count();
                    self.node_registry.invalidate_all();
                    self.scsynth_process = None;
                    self.is_running = false;
                    self.backend = None;
                    self.server_status = ServerStatus::Error;
                    self.groups_created = false;
                    Some(format!(
                        "scsynth exited ({}) \u{2014} {} tracked nodes invalidated",
                        status, live_before
                    ))
                }
                _ => None,
            }
        } else if self.is_running {
            self.node_registry.invalidate_all();
            // is_running but no process — stale state
            self.is_running = false;
            self.server_status = ServerStatus::Error;
            self.groups_created = false;
            Some("scsynth process lost".to_string())
        } else {
            None
        }
    }

    pub fn stop_server(&mut self) {
        self.stop_recording();
        self.disconnect();
        if let Some(child) = self.scsynth_process.take() {
            // Detach: kill+wait can block up to 500ms, so run off the audio thread
            thread::spawn(move || {
                let mut child = child;
                let _ = child.kill();
                let _ = child.wait();
            });
        }
        self.server_status = ServerStatus::Stopped;
    }

    pub fn compile_synthdefs_async(&mut self, scd_path: &Path) -> Result<(), String> {
        if self.is_compiling {
            return Err("Compilation already in progress".to_string());
        }
        if !scd_path.exists() {
            return Err(format!("File not found: {}", scd_path.display()));
        }

        if Self::synthdefs_are_fresh(scd_path) {
            let (tx, rx) = mpsc::channel();
            self.compile_receiver = Some(rx);
            self.is_compiling = true;
            let _ = tx.send(Ok("Synthdefs up-to-date, skipped compilation".to_string()));
            return Ok(());
        }

        let path = scd_path.to_path_buf();
        let (tx, rx) = mpsc::channel();
        self.compile_receiver = Some(rx);
        self.is_compiling = true;

        thread::spawn(move || {
            let result = Self::run_sclang(&path);
            let _ = tx.send(result);
        });

        Ok(())
    }

    /// Check if synthdefs need compilation (returns true if compilation needed).
    /// This is the public wrapper for use during startup.
    pub fn synthdefs_need_compilation(scd_path: &Path) -> bool {
        !Self::synthdefs_are_fresh(scd_path)
    }

    /// Check if all `.scsyndef` files in the same directory as `scd_path` are
    /// newer than the newest `.scd` source file. Scans all `.scd` files
    /// recursively under the parent directory (to cover `defs/` subdirectory).
    /// Returns `true` if compilation can be skipped.
    fn synthdefs_are_fresh(scd_path: &Path) -> bool {
        let dir = match scd_path.parent() {
            Some(d) => d,
            None => return false,
        };

        // Collect all .scd files recursively under the synthdefs directory
        let mut scd_files: Vec<PathBuf> = Vec::new();
        let mut dirs_to_scan = vec![dir.to_path_buf()];
        while let Some(scan_dir) = dirs_to_scan.pop() {
            let entries = match fs::read_dir(&scan_dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("scd") {
                    scd_files.push(path);
                }
            }
        }

        if scd_files.is_empty() {
            return false;
        }

        // Find the newest mtime among all .scd files
        let mut newest_scd_mtime = std::time::SystemTime::UNIX_EPOCH;
        for scd_file in &scd_files {
            if let Ok(mtime) = fs::metadata(scd_file).and_then(|m| m.modified()) {
                if mtime > newest_scd_mtime {
                    newest_scd_mtime = mtime;
                }
            }
        }

        // Extract SynthDef names from all .scd files
        let name_re = match Regex::new(r#"SynthDef\s*\(\s*[\\"]([\w]+)"#) {
            Ok(re) => re,
            Err(_) => return false,
        };

        let mut names: HashSet<String> = HashSet::new();
        for scd_file in &scd_files {
            if let Ok(content) = fs::read_to_string(scd_file) {
                for caps in name_re.captures_iter(&content) {
                    if let Some(name) = caps.get(1).map(|m| m.as_str().to_string()) {
                        names.insert(name);
                    }
                }
            }
        }

        if names.is_empty() {
            return false;
        }

        // Check each .scsyndef is newer than the newest .scd mtime
        for name in names {
            let path = dir.join(format!("{name}.scsyndef"));
            let def_mtime = match fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            if def_mtime <= newest_scd_mtime {
                return false;
            }
        }

        true
    }

    pub fn poll_compile_result(&mut self) -> Option<Result<String, String>> {
        if let Some(ref rx) = self.compile_receiver {
            match rx.try_recv() {
                Ok(result) => {
                    self.compile_receiver = None;
                    self.is_compiling = false;
                    Some(result)
                }
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.compile_receiver = None;
                    self.is_compiling = false;
                    Some(Err("Compilation thread terminated unexpectedly".to_string()))
                }
            }
        } else {
            None
        }
    }

    fn run_sclang(scd_path: &PathBuf) -> Result<String, String> {
        let sclang_paths = [
            "sclang",
            "/Applications/SuperCollider.app/Contents/MacOS/sclang",
            "/usr/local/bin/sclang",
            "/usr/bin/sclang",
        ];

        for path in &sclang_paths {
            match Command::new(path)
                .arg(scd_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
            {
                Ok(output) if output.status.success() => {
                    return Ok("Synthdefs compiled successfully".to_string());
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("Compilation failed: {}", stderr));
                }
                Err(_) => continue,
            }
        }

        Err("Could not find sclang. Install SuperCollider.".to_string())
    }

    pub fn connect(&mut self, server_addr: &str) -> std::io::Result<()> {
        let client = OscClient::new(server_addr)?;
        let backend = ScBackend::new(client);
        backend.send_raw("/notify", vec![RawArg::Int(1)])
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        self.backend = Some(Box::new(backend));
        self.is_running = true;
        self.server_status = ServerStatus::Connected;
        Ok(())
    }

    pub fn connect_with_monitor(&mut self, server_addr: &str, monitor: AudioMonitor) -> std::io::Result<()> {
        let client = OscClient::new_with_monitor(server_addr, monitor)?;
        let backend = ScBackend::new(client);
        backend.send_raw("/notify", vec![RawArg::Int(1)])
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        self.backend = Some(Box::new(backend));
        self.is_running = true;
        self.server_status = ServerStatus::Connected;
        Ok(())
    }

    /// Spawn a background thread that creates the OSC client, backend, and sends /notify.
    /// Returns a receiver that delivers the ready-to-install backend.
    pub(crate) fn connect_with_monitor_async(
        server_addr: String,
        monitor: AudioMonitor,
    ) -> mpsc::Receiver<Result<Box<dyn AudioBackend + Send>, String>> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = (|| -> Result<Box<dyn AudioBackend + Send>, String> {
                let client = OscClient::new_with_monitor(&server_addr, monitor)
                    .map_err(|e| e.to_string())?;
                let backend = ScBackend::new(client);
                backend.send_raw("/notify", vec![RawArg::Int(1)])
                    .map_err(|e| e.to_string())?;
                Ok(Box::new(backend) as Box<dyn AudioBackend + Send>)
            })();
            let _ = tx.send(result);
        });
        rx
    }

    /// Install a fully-initialized backend (created by the connect thread).
    pub(crate) fn install_backend(&mut self, backend: Box<dyn AudioBackend + Send>) {
        self.backend = Some(backend);
        self.is_running = true;
        self.server_status = ServerStatus::Connected;
        self.start_osc_sender();
    }

    pub(super) fn restart_meter(&mut self) {
        if let Some(node_id) = self.meter_node_id.take() {
            self.node_registry.unregister(node_id);
            if let Some(ref backend) = self.backend {
                let _ = backend.free_node(node_id);
            }
        }
        // Free existing analysis synths
        if let Some(ref backend) = self.backend {
            for &node_id in &self.analysis_node_ids {
                self.node_registry.unregister(node_id);
                let _ = backend.free_node(node_id);
            }
        }
        self.analysis_node_ids.clear();

        if let Some(ref backend) = self.backend {
            // Create meter synth (in GROUP_SAFETY so it reads post-limiter signal)
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            let args = vec![
                RawArg::Str("imbolc_meter".to_string()),
                RawArg::Int(node_id),
                RawArg::Int(3), // addAfter
                RawArg::Int(GROUP_SAFETY),
            ];
            if backend.send_raw("/s_new", args).is_ok() {
                self.node_registry.register(node_id);
                self.meter_node_id = Some(node_id);
            }

            // Create analysis synths (spectrum, LUFS, scope) in GROUP_SAFETY
            for synth_def in &["imbolc_spectrum", "imbolc_lufs_meter", "imbolc_scope"] {
                let node_id = self.next_node_id;
                self.next_node_id += 1;
                let args = vec![
                    RawArg::Str(synth_def.to_string()),
                    RawArg::Int(node_id),
                    RawArg::Int(3), // addAfter
                    RawArg::Int(GROUP_SAFETY),
                ];
                if backend.send_raw("/s_new", args).is_ok() {
                    self.node_registry.register(node_id);
                    self.analysis_node_ids.push(node_id);
                }
            }
        }
    }

    /// Send a /status query to SuperCollider (response arrives as /status.reply via OSC)
    pub fn send_status_query(&self) {
        if let Some(ref backend) = self.backend {
            let _ = backend.send_raw("/status", vec![]);
        }
    }

    pub fn disconnect(&mut self) {
        self.stop_osc_sender();
        self.stop_recording();
        if let Some(ref backend) = self.backend {
            if let Some(node_id) = self.safety_node_id.take() {
                let _ = backend.free_node(node_id);
            }
            if let Some(node_id) = self.meter_node_id.take() {
                let _ = backend.free_node(node_id);
            }
            for &node_id in &self.analysis_node_ids {
                let _ = backend.free_node(node_id);
            }
            for nodes in self.node_map.values() {
                for node_id in nodes.all_node_ids() {
                    let _ = backend.free_node(node_id);
                }
            }
            for &node_id in self.bus_effect_node_map.values() {
                let _ = backend.free_node(node_id);
            }
            for &node_id in self.layer_group_effect_node_map.values() {
                let _ = backend.free_node(node_id);
            }
            for &node_id in self.layer_group_eq_node_map.values() {
                let _ = backend.free_node(node_id);
            }
            // Free all loaded sample buffers
            for &bufnum in self.buffer_map.values() {
                let _ = backend.free_buffer(bufnum);
            }
        }
        self.node_map.clear();
        self.send_node_map.clear();
        self.bus_node_map.clear();
        self.bus_effect_node_map.clear();
        self.layer_group_effect_node_map.clear();
        self.layer_group_eq_node_map.clear();
        self.bus_audio_buses.clear();
        // Drain all voices (no OSC needed since server is disconnecting)
        let _ = self.voice_allocator.drain_all();
        // Return oneshot buses to the pool before clearing
        for (_, buses) in self.oneshot_buses.drain() {
            self.voice_allocator.return_control_buses(buses.0, buses.1, buses.2);
        }
        self.analysis_node_ids.clear();
        self.buffer_map.clear();
        self.bus_allocator.reset();
        self.node_registry.invalidate_all();
        self.groups_created = false;
        self.wavetables_initialized = false;
        self.backend = None;
        self.is_running = false;
        if self.scsynth_process.is_some() {
            self.server_status = ServerStatus::Running;
        } else {
            self.server_status = ServerStatus::Stopped;
        }
    }

    pub(super) fn ensure_groups(&mut self) -> Result<(), String> {
        if self.groups_created {
            return Ok(());
        }
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.create_group(GROUP_SOURCES, 1, 0).map_err(|e| e.to_string())?;
        backend.create_group(GROUP_PROCESSING, 1, 0).map_err(|e| e.to_string())?;
        backend.create_group(GROUP_OUTPUT, 1, 0).map_err(|e| e.to_string())?;
        backend.create_group(GROUP_BUS_PROCESSING, 1, 0).map_err(|e| e.to_string())?;
        backend.create_group(GROUP_RECORD, 1, 0).map_err(|e| e.to_string())?;
        backend.create_group(GROUP_SAFETY, 1, 0).map_err(|e| e.to_string())?;
        self.groups_created = true;
        Ok(())
    }

    pub(super) fn ensure_safety_limiter(&mut self) -> Result<(), String> {
        if self.safety_node_id.is_some() {
            return Ok(());
        }
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        let args = vec![
            RawArg::Str("imbolc_safety".to_string()),
            RawArg::Int(node_id),
            RawArg::Int(0), // addToHead
            RawArg::Int(GROUP_SAFETY),
            RawArg::Str("ceiling".to_string()),
            RawArg::Float(0.95),
        ];
        if backend.send_raw("/s_new", args).is_ok() {
            self.node_registry.register(node_id);
            self.safety_node_id = Some(node_id);
        }
        Ok(())
    }

    /// Connect SuperCollider's JACK output ports to the first available
    /// hardware playback ports. Uses `pw-jack jack_lsp` to discover port
    /// names and `pw-jack jack_connect` to wire them up.
    /// Spawns a background thread to avoid blocking startup — scsynth may
    /// need a few seconds to register its JACK ports with PipeWire.
    fn connect_jack_ports() {
        thread::spawn(|| {
            // Wait for SuperCollider JACK ports to appear (up to 5s)
            let mut sc_ports_ready = false;
            for _ in 0..25 {
                thread::sleep(Duration::from_millis(200));
                if let Ok(output) = Command::new("pw-jack")
                    .args(["jack_lsp"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output()
                {
                    let text = String::from_utf8_lossy(&output.stdout);
                    if text.lines().any(|l| l == "SuperCollider:out_1") {
                        sc_ports_ready = true;
                        break;
                    }
                }
            }
            if !sc_ports_ready {
                return;
            }

            // Discover hardware playback ports
            let playback_ports: Vec<String> = Command::new("pw-jack")
                .args(["jack_lsp"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .map(|output| {
                    let text = String::from_utf8_lossy(&output.stdout);
                    text.lines()
                        .filter(|l| !l.starts_with("SuperCollider:"))
                        .filter(|l| l.contains(":playback_"))
                        .take(2)
                        .map(|l| l.to_string())
                        .collect()
                })
                .unwrap_or_default();

            let sc_outs = ["SuperCollider:out_1", "SuperCollider:out_2"];
            for (sc_port, hw_port) in sc_outs.iter().zip(playback_ports.iter()) {
                let _ = Command::new("pw-jack")
                    .args(["jack_connect", sc_port, hw_port])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        });
    }
}
