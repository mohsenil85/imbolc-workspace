// Re-export core crate modules so crate::state, crate::audio, etc. resolve throughout the binary
pub use imbolc_audio as audio;
pub use imbolc_core::action;
pub use imbolc_core::config;
pub use imbolc_core::dispatch;
pub use imbolc_core::midi;
pub use imbolc_core::scd_parser;
pub use imbolc_core::state;

mod global_actions;
mod midi_dispatch;
#[cfg(feature = "net")]
mod network;
mod panes;
mod runtime;
mod setup;
mod ui;

use std::fs::File;

use panes::{
    AddEffectPane, AddPane, ArpeggiatorPane, AutomationPane, CheckpointListPane,
    CommandPalettePane, ConfirmPane, DocsPane, EqPane, FileBrowserPane, FrameEditPane, GroovePane,
    HelpPane, HomePane, InstrumentEditPane, InstrumentPane, InstrumentPickerPane,
    MidiSettingsPane, MixerPane, PaneSwitcherPane, PianoRollPane, ProjectBrowserPane,
    QuitPromptPane, SampleChopperPane, SaveAsPane, SequencerPane, ServerPane, TrackPane,
    TunerPane, VstParamPane, WaveformPane,
};
use ui::{Keymap, PaneManager, RatatuiBackend};

fn init_logging(verbose: bool) {
    use simplelog::*;

    let log_level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Warn
    };

    let log_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("imbolc")
        .join("imbolc.log");

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let log_file = File::create(&log_path)
        .unwrap_or_else(|_| File::create("/tmp/imbolc.log").expect("Cannot create log file"));

    WriteLogger::init(log_level, Config::default(), log_file).expect("Failed to initialize logger");

    log::info!("imbolc starting (log level: {:?})", log_level);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
    init_logging(verbose);

    // Check for network modes
    let server_mode = args.iter().any(|a| a == "--server");
    let _discover_mode = args.iter().any(|a| a == "--discover");
    let connect_addr = args
        .iter()
        .position(|a| a == "--connect")
        .and_then(|i| args.get(i + 1).cloned());

    // Parse --own flag for ownership requests (comma-separated instrument IDs)
    let own_instruments: Vec<u32> = args
        .iter()
        .position(|a| a == "--own")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_default();

    #[cfg(feature = "net")]
    {
        if server_mode {
            return network::run_server();
        }
        if discover_mode {
            #[cfg(feature = "mdns")]
            return network::run_discovery(own_instruments);
            #[cfg(not(feature = "mdns"))]
            {
                eprintln!("Discovery mode requires the 'mdns' feature. Build with: cargo build --features mdns");
                std::process::exit(1);
            }
        }
        if let Some(addr) = connect_addr {
            return network::run_client(&addr, own_instruments);
        }
    }

    #[cfg(not(feature = "net"))]
    {
        let _ = own_instruments; // Silence unused warning when net feature disabled
        if server_mode || connect_addr.is_some() {
            eprintln!(
                "Network mode requires the 'net' feature. Build with: cargo build --features net"
            );
            std::process::exit(1);
        }
    }

    // Install panic hook to restore terminal before printing panic info
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort terminal restoration — ignore errors
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::event::PopKeyboardEnhancementFlags,
        );
        // Print panic info to stderr, then delegate to original hook (backtrace etc.)
        original_hook(info);
    }));

    let mut backend = RatatuiBackend::new()?;
    backend.start()?;

    let result = runtime::run(&mut backend);

    backend.stop()?;
    result
}

pub(crate) fn pane_keymap(
    keymaps: &mut std::collections::HashMap<String, Keymap>,
    id: &str,
) -> Keymap {
    keymaps.remove(id).unwrap_or_default()
}

pub(crate) fn register_all_panes(
    keymaps: &mut std::collections::HashMap<String, Keymap>,
) -> PaneManager {
    // file_browser keymap is used by both FileBrowserPane and SampleChopperPane's internal browser
    let file_browser_km = keymaps
        .get("file_browser")
        .cloned()
        .unwrap_or_else(Keymap::new);

    let mut panes = PaneManager::new(Box::new(InstrumentEditPane::new(pane_keymap(
        keymaps,
        "instrument_edit",
    ))));
    panes.add_pane(Box::new(HomePane::new(pane_keymap(keymaps, "home"))));
    panes.add_pane(Box::new(AddPane::new(pane_keymap(keymaps, "add"))));
    panes.add_pane(Box::new(InstrumentPane::new(pane_keymap(
        keymaps,
        "instrument",
    ))));
    panes.add_pane(Box::new(ServerPane::new(pane_keymap(keymaps, "server"))));
    panes.add_pane(Box::new(MixerPane::new(pane_keymap(keymaps, "mixer"))));
    panes.add_pane(Box::new(HelpPane::new(pane_keymap(keymaps, "help"))));
    panes.add_pane(Box::new(PianoRollPane::new(pane_keymap(
        keymaps,
        "piano_roll",
    ))));
    panes.add_pane(Box::new(SequencerPane::new(pane_keymap(
        keymaps,
        "sequencer",
    ))));
    panes.add_pane(Box::new(FrameEditPane::new(pane_keymap(
        keymaps,
        "frame_edit",
    ))));
    panes.add_pane(Box::new(SampleChopperPane::new(
        pane_keymap(keymaps, "sample_chopper"),
        file_browser_km,
    )));
    panes.add_pane(Box::new(AddEffectPane::new(pane_keymap(
        keymaps,
        "add_effect",
    ))));
    panes.add_pane(Box::new(InstrumentPickerPane::new(pane_keymap(
        keymaps, "add",
    ))));
    panes.add_pane(Box::new(FileBrowserPane::new(pane_keymap(
        keymaps,
        "file_browser",
    ))));
    panes.add_pane(Box::new(TrackPane::new(pane_keymap(keymaps, "track"))));
    panes.add_pane(Box::new(WaveformPane::new(pane_keymap(
        keymaps, "waveform",
    ))));
    panes.add_pane(Box::new(AutomationPane::new(pane_keymap(
        keymaps,
        "automation",
    ))));
    panes.add_pane(Box::new(EqPane::new(pane_keymap(keymaps, "eq"))));
    panes.add_pane(Box::new(GroovePane::new(pane_keymap(keymaps, "groove"))));
    panes.add_pane(Box::new(ArpeggiatorPane::new(pane_keymap(
        keymaps,
        "arpeggiator",
    ))));
    panes.add_pane(Box::new(VstParamPane::new(pane_keymap(
        keymaps,
        "vst_params",
    ))));
    panes.add_pane(Box::new(ConfirmPane::new(pane_keymap(keymaps, "confirm"))));
    panes.add_pane(Box::new(QuitPromptPane::new(pane_keymap(
        keymaps,
        "quit_prompt",
    ))));
    panes.add_pane(Box::new(ProjectBrowserPane::new(pane_keymap(
        keymaps,
        "project_browser",
    ))));
    panes.add_pane(Box::new(SaveAsPane::new(pane_keymap(keymaps, "save_as"))));
    panes.add_pane(Box::new(CommandPalettePane::new(pane_keymap(
        keymaps,
        "command_palette",
    ))));
    panes.add_pane(Box::new(PaneSwitcherPane::new(pane_keymap(
        keymaps,
        "pane_switcher",
    ))));
    panes.add_pane(Box::new(MidiSettingsPane::new(pane_keymap(
        keymaps,
        "midi_settings",
    ))));
    panes.add_pane(Box::new(TunerPane::new(pane_keymap(keymaps, "tuner"))));
    panes.add_pane(Box::new(DocsPane::new(pane_keymap(keymaps, "docs"))));
    panes.add_pane(Box::new(CheckpointListPane::new(pane_keymap(
        keymaps,
        "checkpoint_list",
    ))));
    panes
}
