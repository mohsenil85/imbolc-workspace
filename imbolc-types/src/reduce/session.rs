use crate::{InstrumentState, SessionAction, SessionState};

pub(super) fn reduce(
    action: &SessionAction,
    session: &mut SessionState,
    _instruments: &mut InstrumentState,
) -> bool {
    match action {
        SessionAction::NewProject => false,
        SessionAction::UpdateSession(ref settings) => {
            session.apply_musical_settings(settings);
            true
        }
        SessionAction::UpdateSessionLive(ref settings) => {
            session.apply_musical_settings(settings);
            true
        }
        SessionAction::AdjustHumanizeVelocity(delta) => {
            session.humanize.velocity = (session.humanize.velocity + delta).clamp(0.0, 1.0);
            true
        }
        SessionAction::AdjustHumanizeTiming(delta) => {
            session.humanize.timing = (session.humanize.timing + delta).clamp(0.0, 1.0);
            true
        }
        SessionAction::ToggleMasterMute => {
            session.mixer.master_mute = !session.mixer.master_mute;
            true
        }
        SessionAction::CycleTheme => {
            use crate::state::Theme;
            let current_name = &session.theme.name;
            session.theme = match current_name.as_str() {
                "Dark" => Theme::light(),
                "Light" => Theme::high_contrast(),
                _ => Theme::dark(),
            };
            true
        }
        SessionAction::ImportVstPlugin(ref path, kind) => {
            use crate::state::vst::VstPlugin;
            let name = path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "VST Plugin".to_string());
            let plugin = VstPlugin {
                id: crate::VstPluginId::new(0),
                name,
                plugin_path: path.clone(),
                kind: *kind,
                params: vec![],
            };
            session.vst_plugins.add(plugin);
            true
        }
        // OpenFileBrowser: navigation only
        SessionAction::OpenFileBrowser(_) => true,
        // File I/O actions: not reducible
        SessionAction::Save
        | SessionAction::SaveAs(_)
        | SessionAction::Load
        | SessionAction::LoadFrom(_)
        | SessionAction::ImportCustomSynthDef(_)
        | SessionAction::CreateCheckpoint(_)
        | SessionAction::RestoreCheckpoint(_)
        | SessionAction::DeleteCheckpoint(_) => false,
    }
}
