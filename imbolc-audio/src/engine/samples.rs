use std::path::Path;

use super::backend::RawArg;
use super::AudioEngine;
use imbolc_types::BufferId;

impl AudioEngine {
    pub fn load_synthdefs(&self, dir: &Path) -> Result<(), String> {
        log::debug!(target: "audio::samples", "load_synthdefs called with: {:?}", dir);
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        log::debug!(target: "audio::samples", "Backend available");

        let abs_dir = dir
            .canonicalize()
            .map_err(|e| format!("Cannot resolve synthdef dir {:?}: {}", dir, e))?;
        log::debug!(target: "audio::samples", "Canonicalized to: {:?}", abs_dir);
        let dir_str = abs_dir
            .to_str()
            .ok_or_else(|| "Synthdef dir path is not valid UTF-8".to_string())?;

        backend
            .send_raw("/d_loadDir", vec![RawArg::Str(dir_str.to_string())])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load a single .scsyndef file into the server
    pub fn load_synthdef_file(&self, path: &Path) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;

        let abs_path = path
            .canonicalize()
            .map_err(|e| format!("Cannot resolve synthdef file {:?}: {}", path, e))?;
        let path_str = abs_path
            .to_str()
            .ok_or_else(|| "Synthdef file path is not valid UTF-8".to_string())?;

        backend
            .send_raw("/d_load", vec![RawArg::Str(path_str.to_string())])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // =========================================================================
    // Wavetable Initialization (for VOsc / imbolc_wavetable)
    // =========================================================================

    /// Allocate and fill wavetable buffers 100–107 so VOsc has data to read.
    /// Each buffer gets a different harmonic spectrum for smooth morphing.
    pub fn initialize_wavetables(&mut self) -> Result<(), String> {
        if self.wavetables_initialized {
            return Ok(());
        }

        let backend = self.backend.as_ref().ok_or("Not connected")?;

        // Harmonic amplitude tables: index 0 = pure sine, 7 = full spectrum
        let tables: Vec<Vec<f32>> = vec![
            // 100: Pure sine
            vec![1.0],
            // 101: Soft
            vec![1.0, 0.5],
            // 102: Warm
            vec![1.0, 0.5, 0.25, 0.125],
            // 103: Medium
            vec![1.0, 0.75, 0.5, 0.35, 0.25, 0.15, 0.1, 0.05],
            // 104: Square-ish (odd harmonics)
            vec![1.0, 0.0, 0.33, 0.0, 0.2, 0.0, 0.14, 0.0, 0.11],
            // 105: Saw-ish (1/n series, 16 harmonics)
            (1..=16).map(|n| 1.0 / n as f32).collect(),
            // 106: Bright (emphasised upper harmonics)
            (1..=16)
                .map(|n| {
                    let x = n as f32 / 16.0;
                    (1.0 - x) * 0.3 + x * 1.0
                })
                .collect(),
            // 107: Full (32 harmonics, gradual decrease)
            (1..=32).map(|n| 1.0 / (n as f32).sqrt()).collect(),
        ];

        for (i, harmonics) in tables.iter().enumerate() {
            let bufnum = super::WAVETABLE_BUFNUM_START + i as i32;

            // /b_alloc bufnum 2048 1
            backend
                .send_raw(
                    "/b_alloc",
                    vec![RawArg::Int(bufnum), RawArg::Int(2048), RawArg::Int(1)],
                )
                .map_err(|e| format!("b_alloc buf {}: {}", bufnum, e))?;

            // /b_gen bufnum "sine1" 7 amp1 amp2 ...
            // flags 7 = normalize(1) + wavetable(2) + clear(4)
            let mut args: Vec<RawArg> = vec![
                RawArg::Int(bufnum),
                RawArg::Str("sine1".to_string()),
                RawArg::Int(7),
            ];
            for &amp in harmonics {
                args.push(RawArg::Float(amp));
            }
            backend
                .send_raw("/b_gen", args)
                .map_err(|e| format!("b_gen buf {}: {}", bufnum, e))?;
        }

        self.wavetables_initialized = true;
        Ok(())
    }

    // =========================================================================
    // Buffer Management (for Sampler)
    // =========================================================================

    /// Load a sample file into a SuperCollider buffer
    /// Returns the SC buffer number on success
    #[allow(dead_code)]
    pub fn load_sample(&mut self, buffer_id: BufferId, path: &str) -> Result<i32, String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;

        // Check if already loaded
        if let Some(&bufnum) = self.buffer_map.get(&buffer_id) {
            return Ok(bufnum);
        }

        let bufnum = self.next_bufnum;
        self.next_bufnum += 1;

        backend
            .load_buffer(bufnum, Path::new(path))
            .map_err(|e| e.to_string())?;

        self.buffer_map.insert(buffer_id, bufnum);
        Ok(bufnum)
    }

    /// Free a sample buffer from SuperCollider
    #[allow(dead_code)]
    pub fn free_sample(&mut self, buffer_id: BufferId) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;

        if let Some(bufnum) = self.buffer_map.remove(&buffer_id) {
            backend.free_buffer(bufnum).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Get the SuperCollider buffer number for a loaded buffer
    #[allow(dead_code)]
    pub fn get_sc_bufnum(&self, buffer_id: BufferId) -> Option<i32> {
        self.buffer_map.get(&buffer_id).copied()
    }

    /// Check if a buffer is loaded
    #[allow(dead_code)]
    pub fn is_buffer_loaded(&self, buffer_id: BufferId) -> bool {
        self.buffer_map.contains_key(&buffer_id)
    }
}
