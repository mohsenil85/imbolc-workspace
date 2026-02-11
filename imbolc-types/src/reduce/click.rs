use crate::{ClickAction, SessionState};

pub(super) fn reduce(action: &ClickAction, session: &mut SessionState) {
    match action {
        ClickAction::Toggle => {
            session.click_track.enabled = !session.click_track.enabled;
        }
        ClickAction::ToggleMute => {
            session.click_track.muted = !session.click_track.muted;
        }
        ClickAction::AdjustVolume(delta) => {
            session.click_track.volume = (session.click_track.volume + delta).clamp(0.0, 1.0);
        }
        ClickAction::SetVolume(volume) => {
            session.click_track.volume = volume.clamp(0.0, 1.0);
        }
    }
}
