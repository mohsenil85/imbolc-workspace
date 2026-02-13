//! Feedback draining: I/O completion callbacks, audio feedback, MIDI events.

use std::time::Instant;

use super::AppRuntime;
use crate::action::{self, AudioEffect, IoFeedback};
use crate::audio::commands::AudioCmd;
use crate::global_actions::apply_dispatch_result;
use crate::panes::ServerPane;
use crate::state;
use crate::ui::status_bar::StatusLevel;

impl AppRuntime {
    /// Drain I/O feedback (save/load/import completions).
    pub(crate) fn drain_io_feedback(&mut self) {
        while let Ok(feedback) = self.io_rx.try_recv() {
            match feedback {
                IoFeedback::SaveComplete { id, path, result } => {
                    if id != self.dispatcher.state().io.generation.save {
                        continue;
                    }
                    let status = match result {
                        Ok(name) => {
                            let state = self.dispatcher.state_mut();
                            state.project.path = Some(path.clone());
                            state.project.dirty = false;
                            self.recent_projects.add(&path, &name);
                            self.recent_projects.save();
                            self.app_frame.set_project_name(name);
                            self.app_frame
                                .status_bar
                                .push("Project saved", StatusLevel::Info);
                            // Invalidate any in-flight autosave write and remove stale snapshot.
                            self.autosave_id = self.autosave_id.wrapping_add(1);
                            self.autosave_in_progress = false;
                            // Explicit saves supersede crash snapshots.
                            let _ = std::fs::remove_file(&self.autosave_path);
                            self.last_autosave_at = Instant::now();
                            "Saved project".to_string()
                        }
                        Err(e) => {
                            let msg = format!("Save failed: {}", e);
                            self.app_frame.status_bar.push(&msg, StatusLevel::Error);
                            msg
                        }
                    };
                    if let Some(server) = self.panes.get_pane_mut::<ServerPane>("server") {
                        server.set_status(self.audio.status(), &status);
                    }
                }
                IoFeedback::AutosaveComplete { id, path, result } => {
                    if id != self.autosave_id {
                        // Discard stale autosave artifact from superseded generation.
                        let _ = std::fs::remove_file(path);
                        continue;
                    }
                    self.autosave_in_progress = false;
                    match result {
                        Ok(()) => {}
                        Err(e) => {
                            let msg = format!("Autosave failed: {}", e);
                            self.app_frame.status_bar.push(&msg, StatusLevel::Error);
                        }
                    }
                }
                IoFeedback::LoadComplete { id, path, result } => {
                    if id != self.dispatcher.state().io.generation.load {
                        continue;
                    }
                    match result {
                        Ok((new_session, new_instruments, name)) => {
                            let recovering_autosave = path == self.autosave_path;
                            {
                                let state = self.dispatcher.state_mut();
                                state.undo_history.clear();
                                state.session = new_session;
                                state.instruments = new_instruments;
                                state.instruments.rebuild_index();
                                if recovering_autosave {
                                    // Keep recovered session unsaved so users pick a real project path.
                                    state.project.path = None;
                                    state.project.dirty = true;
                                } else {
                                    state.project.path = Some(path.clone());
                                    state.project.dirty = false;
                                }
                            }
                            if recovering_autosave {
                                self.app_frame.set_project_name("autosave-recovered".to_string());
                            } else {
                                self.recent_projects.add(&path, &name);
                                self.recent_projects.save();
                                self.app_frame.set_project_name(name);
                            }
                            self.last_autosave_at = Instant::now();
                            self.autosave_id = self.autosave_id.wrapping_add(1);
                            self.autosave_in_progress = false;

                            if self.dispatcher.state().instruments.instruments.is_empty() {
                                self.panes
                                    .switch_to(action::PaneId::Add, self.dispatcher.state());
                            }

                            self.pending_audio_effects.extend(AudioEffect::all());
                            self.needs_full_sync = true;

                            // Queue VST state restores
                            let vst_restores: Vec<_> = self
                                .dispatcher
                                .state()
                                .instruments
                                .instruments
                                .iter()
                                .flat_map(|inst| {
                                    let mut restores = Vec::new();
                                    if inst.source.is_vst() {
                                        if let Some(path) = inst.vst_source_state_path() {
                                            restores.push((
                                                inst.id,
                                                action::VstTarget::Source,
                                                path.clone(),
                                            ));
                                        }
                                    }
                                    for effect in inst.effects() {
                                        if let (state::EffectType::Vst(_), Some(ref path)) =
                                            (&effect.effect_type, &effect.vst_state_path)
                                        {
                                            restores.push((
                                                inst.id,
                                                action::VstTarget::Effect(effect.id),
                                                path.clone(),
                                            ));
                                        }
                                    }
                                    restores
                                })
                                .collect();

                            for (instrument_id, target, path) in vst_restores {
                                let _ = self.audio.send_cmd(AudioCmd::LoadVstState {
                                    instrument_id,
                                    target,
                                    path,
                                });
                            }

                            let status_msg = if recovering_autosave {
                                "Recovered autosave snapshot (unsaved)".to_string()
                            } else {
                                "Project loaded".to_string()
                            };
                            let status_level = if recovering_autosave {
                                StatusLevel::Warning
                            } else {
                                StatusLevel::Info
                            };
                            self.app_frame.status_bar.push(&status_msg, status_level);
                            if let Some(server) = self.panes.get_pane_mut::<ServerPane>("server") {
                                server.set_status(self.audio.status(), &status_msg);
                            }
                        }
                        Err(e) => {
                            let msg = format!("Load failed: {}", e);
                            self.app_frame.status_bar.push(&msg, StatusLevel::Error);
                            if let Some(server) = self.panes.get_pane_mut::<ServerPane>("server") {
                                server.set_status(self.audio.status(), &msg);
                            }
                        }
                    }
                }
                IoFeedback::ImportSynthDefComplete { id, result } => {
                    if id != self.dispatcher.state().io.generation.import_synthdef {
                        continue;
                    }
                    match result {
                        Ok((custom, synthdef_name, scsyndef_path)) => {
                            let _id = self
                                .dispatcher
                                .state_mut()
                                .session
                                .custom_synthdefs
                                .add(custom);
                            self.pending_audio_effects.push(AudioEffect::RebuildSession);
                            self.needs_full_sync = true;

                            if self.audio.is_running() {
                                if let Some(server) =
                                    self.panes.get_pane_mut::<ServerPane>("server")
                                {
                                    server.set_status(
                                        self.audio.status(),
                                        &format!("Loading custom synthdef: {}", synthdef_name),
                                    );
                                }
                                if let Err(e) = self.audio.load_synthdef_file(&scsyndef_path) {
                                    if let Some(server) =
                                        self.panes.get_pane_mut::<ServerPane>("server")
                                    {
                                        server.set_status(
                                            self.audio.status(),
                                            &format!("Failed to load synthdef: {}", e),
                                        );
                                    }
                                }
                            } else if let Some(server) =
                                self.panes.get_pane_mut::<ServerPane>("server")
                            {
                                server.set_status(
                                    self.audio.status(),
                                    &format!("Imported custom synthdef: {}", synthdef_name),
                                );
                            }
                        }
                        Err(e) => {
                            if let Some(server) = self.panes.get_pane_mut::<ServerPane>("server") {
                                server.set_status(
                                    self.audio.status(),
                                    &format!("Import error: {}", e),
                                );
                            }
                        }
                    }
                }
                IoFeedback::ImportSynthDefLoaded { id, result } => {
                    if id != self.dispatcher.state().io.generation.import_synthdef {
                        continue;
                    }
                    let status = match result {
                        Ok(name) => format!("Loaded custom synthdef: {}", name),
                        Err(e) => format!("Failed to load synthdef: {}", e),
                    };
                    if let Some(server) = self.panes.get_pane_mut::<ServerPane>("server") {
                        server.set_status(self.audio.status(), &status);
                    }
                }
            }
        }
    }

    /// Drain audio feedback (playhead, meters, status updates).
    pub(crate) fn drain_audio_feedback(&mut self) {
        for feedback in self.audio.drain_feedback() {
            self.render_needed = true;
            let mut r = self.dispatcher.dispatch_domain(
                &action::DomainAction::AudioFeedback(feedback),
                &mut self.audio,
            );
            if r.needs_full_sync {
                self.needs_full_sync = true;
            }
            self.pending_audio_effects
                .extend(std::mem::take(&mut r.audio_effects));
            apply_dispatch_result(
                r,
                &mut self.dispatcher,
                &mut self.panes,
                &mut self.app_frame,
                &mut self.audio,
            );
        }
    }

    /// Poll MIDI events and dispatch them.
    pub(crate) fn drain_midi_events(&mut self) {
        use imbolc_types::RoutedAction;
        for event in self.midi_input.poll_events() {
            if let Some(action) =
                crate::midi_dispatch::process_midi_event(&event, self.dispatcher.state())
            {
                self.render_needed = true;
                if let RoutedAction::Domain(ref domain) = action.route() {
                    let r = self.dispatcher.dispatch_domain(domain, &mut self.audio);
                    if r.needs_full_sync {
                        self.needs_full_sync = true;
                    }
                    self.pending_audio_effects.extend(r.audio_effects);
                }
            }
        }
    }
}
