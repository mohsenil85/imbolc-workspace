//! Audio effect synchronization.

use super::AppRuntime;

impl AppRuntime {
    /// Apply pending audio effects to the audio engine.
    pub(crate) fn apply_pending_effects(&mut self) {
        if !self.pending_audio_effects.is_empty() {
            self.audio.apply_effects(
                self.dispatcher.state(),
                &self.pending_audio_effects,
                self.needs_full_sync,
            );
            self.pending_audio_effects.clear();
            self.needs_full_sync = false;
        }
    }
}
