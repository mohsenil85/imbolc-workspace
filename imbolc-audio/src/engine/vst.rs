use super::backend::RawArg;
use super::AudioEngine;
use super::VST_UGEN_INDEX;
use imbolc_types::InstrumentId;

impl AudioEngine {
    /// Send MIDI note-on to a VSTi persistent source node
    pub(super) fn send_vsti_note_on(&self, instrument_id: InstrumentId, pitch: u8, velocity: f32) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(source_node) = nodes.source {
                let vel = (velocity * 127.0).round().min(127.0) as u8;
                // MIDI note-on: status 0x90, note, velocity as raw bytes
                let midi_msg: Vec<u8> = vec![0x90, pitch, vel];
                backend.send_unit_cmd(
                    source_node,
                    VST_UGEN_INDEX,
                    "/midi_msg",
                    vec![RawArg::Blob(midi_msg)],
                ).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Send MIDI note-off to a VSTi persistent source node
    pub(super) fn send_vsti_note_off(&self, instrument_id: InstrumentId, pitch: u8) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        if let Some(nodes) = self.node_map.get(&instrument_id) {
            if let Some(source_node) = nodes.source {
                // MIDI note-off: status 0x80, note, velocity 0
                let midi_msg: Vec<u8> = vec![0x80, pitch, 0];
                backend.send_unit_cmd(
                    source_node,
                    VST_UGEN_INDEX,
                    "/midi_msg",
                    vec![RawArg::Blob(midi_msg)],
                ).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Query VST parameter count from a VST node
    #[allow(dead_code)]
    pub(crate) fn query_vst_param_count_node(&self, node_id: i32) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_unit_cmd(
            node_id,
            VST_UGEN_INDEX,
            "/param_count",
            vec![],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Query VST parameter info for a specific index
    #[allow(dead_code)] // Reserved for future SC reply handling
    pub(crate) fn query_vst_param_info_node(&self, node_id: i32, index: u32) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_unit_cmd(
            node_id,
            VST_UGEN_INDEX,
            "/param_info",
            vec![RawArg::Int(index as i32)],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Query a range of VST parameters via /param_query.
    /// VSTPlugin replies with /vst_param messages via SendNodeReply for each param in range.
    pub(crate) fn query_vst_params_range(&self, node_id: i32, start: u32, count: u32) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_unit_cmd(
            node_id,
            VST_UGEN_INDEX,
            "/param_query",
            vec![RawArg::Int(start as i32), RawArg::Int(count as i32)],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Set a VST parameter value on a resolved node
    pub(crate) fn set_vst_param_node(&self, node_id: i32, param_index: u32, value: f32) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_unit_cmd(
            node_id,
            VST_UGEN_INDEX,
            "/set",
            vec![RawArg::Int(param_index as i32), RawArg::Float(value)],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Save VST plugin state to a file from a resolved node
    pub(crate) fn save_vst_state_node(&self, node_id: i32, path: &std::path::Path) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_unit_cmd(
            node_id,
            VST_UGEN_INDEX,
            "/program_write",
            vec![RawArg::Str(path.to_string_lossy().to_string())],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load VST plugin state from a file into a resolved node
    pub(crate) fn load_vst_state_node(&self, node_id: i32, path: &std::path::Path) -> Result<(), String> {
        let backend = self.backend.as_ref().ok_or("Not connected")?;
        backend.send_unit_cmd(
            node_id,
            VST_UGEN_INDEX,
            "/program_read",
            vec![RawArg::Str(path.to_string_lossy().to_string())],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }
}
