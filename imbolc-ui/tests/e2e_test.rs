mod e2e;

use e2e::TmuxHarness;
use std::time::Duration;

/// Path to the built binary
fn binary_path() -> String {
    let path = format!(
        "{}/target/debug/imbolc",
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string())
    );
    assert!(
        std::path::Path::new(&path).exists(),
        "Binary not found at {}. Run `cargo build` first.",
        path
    );
    path
}

/// Check if tmux is available, skip test if not
fn require_tmux() -> bool {
    std::process::Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Small sleep to let the TUI re-render after a keypress
fn wait_render() {
    std::thread::sleep(Duration::from_millis(200));
}

/// Start the app and add a default (Saw) instrument so tests begin at the
/// Instruments pane rather than the Add Instrument dialog.
fn start_with_instrument(test_name: &str) -> TmuxHarness {
    let harness = TmuxHarness::new(test_name);
    harness.start(&binary_path()).expect("Failed to start app");
    wait_render();
    // Press Enter to confirm the default Saw source in the Add dialog
    harness.send_key("Enter").expect("Failed to send Enter");
    wait_render();
    harness
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_displays_box_with_title() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = TmuxHarness::new("box-title");
    harness.start(&binary_path()).expect("Failed to start app");
    wait_render();

    // App starts with no instruments → AddPane is shown
    harness
        .assert_screen_contains("Add Instrument")
        .expect("Should display 'Add Instrument' dialog");

    // Verify the frame header renders
    harness
        .assert_screen_contains("IMBOLC")
        .expect("Should display 'IMBOLC' frame header");

    // Verify box borders are present
    let screen = harness.capture_screen().expect("Should capture screen");
    assert!(
        screen.contains("┌") || screen.contains("+") || screen.contains("╭"),
        "Should display box border\nScreen:\n{}",
        screen
    );
}

#[test]
fn test_quit_with_q() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = TmuxHarness::new("quit");
    harness.start(&binary_path()).expect("Failed to start app");
    wait_render();

    assert!(harness.is_running(), "App should be running initially");

    harness.send_key("C-q").expect("Failed to send 'C-q'");

    harness
        .wait_for_exit(Duration::from_secs(3))
        .expect("App should exit after pressing Ctrl+q");

    assert!(!harness.is_running(), "App should have exited");
}

#[test]
fn test_add_instrument_list_shows_sources() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = TmuxHarness::new("add-sources");
    harness.start(&binary_path()).expect("Failed to start app");
    wait_render();

    // The Add Instrument dialog lists oscillator source types
    harness
        .assert_screen_contains("Saw")
        .expect("Should list Saw source");
    harness
        .assert_screen_contains("Sin")
        .expect("Should list Sin source");
}

#[test]
fn test_add_instrument_via_enter() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("add-enter");

    // After adding, we land on the Edit pane
    harness
        .assert_screen_contains("Edit:")
        .expect("Should show Edit pane after adding");

    // The Add Instrument dialog should be gone
    let screen = harness.capture_screen().expect("capture");
    assert!(
        !screen.contains("Add Instrument"),
        "Add Instrument dialog should be dismissed\nScreen:\n{}",
        screen
    );
}

#[test]
fn test_escape_from_add_returns() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("esc-add");

    // Ctrl+g to instrument list, then 'a' to add
    harness.send_key("C-g").expect("send C-g");
    wait_render();
    harness.send_key("a").expect("send a");
    wait_render();
    harness
        .assert_screen_contains("Add Instrument")
        .expect("Add dialog should reopen");

    // Escape cancels back to Instruments
    harness.send_key("Escape").expect("send Escape");
    wait_render();
    let screen = harness.capture_screen().expect("capture");
    assert!(
        !screen.contains("Add Instrument"),
        "Add dialog should close on Escape\nScreen:\n{}",
        screen
    );
}

#[test]
fn test_f1_shows_instrument_pane() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("f1-inst");

    // Navigate away first (F4 to mixer)
    harness.send_key("F4").expect("send F4");
    wait_render();

    // F1 back to instrument edit
    harness.send_key("F1").expect("send F1");
    wait_render();
    harness
        .assert_screen_contains("SOURCE:")
        .expect("F1 should show instrument edit pane");
}

#[test]
fn test_f4_shows_mixer_pane() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("f4-mixer");

    harness.send_key("F4").expect("send F4");
    wait_render();
    harness
        .assert_screen_contains("MIXER")
        .expect("F4 should show MIXER pane");
}

#[test]
fn test_f5_shows_server_pane() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("f5-server");

    harness.send_key("F5").expect("send F5");
    wait_render();
    harness
        .assert_screen_contains("Audio Server")
        .expect("F5 should show Audio Server pane");

    // With IMBOLC_NO_AUDIO, server should report Stopped
    harness
        .assert_screen_contains("Stopped")
        .expect("Server should be stopped without audio");
}

#[test]
fn test_ctrl_f_opens_and_closes_frame_edit() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("frame-edit");

    // Ctrl+f opens the Session modal
    harness.send_key("C-f").expect("send C-f");
    wait_render();
    harness
        .assert_screen_contains("Session")
        .expect("Ctrl+f should open Session modal");
    harness
        .assert_screen_contains("BPM")
        .expect("Session modal should show BPM field");

    // Escape closes it
    harness.send_key("Escape").expect("send Escape");
    wait_render();
    let screen = harness.capture_screen().expect("capture");
    assert!(
        !screen.contains("Session"),
        "Session modal should close on Escape\nScreen:\n{}",
        screen
    );
}

#[test]
fn test_help_modal() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("help");

    // ? opens context-sensitive help
    harness.send_key("?").expect("send ?");
    wait_render();
    harness
        .assert_screen_contains("Help")
        .expect("? should open Help modal");

    // Escape closes it
    harness.send_key("Escape").expect("send Escape");
    wait_render();
    let screen = harness.capture_screen().expect("capture");
    assert!(
        !screen.contains("Help:"),
        "Help modal should close on Escape\nScreen:\n{}",
        screen
    );
}

#[test]
fn test_navigate_between_panes() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("nav-panes");

    // F4 → Mixer
    harness.send_key("F4").expect("send F4");
    wait_render();
    harness
        .assert_screen_contains("MIXER")
        .expect("Should be on Mixer");

    // F5 → Server
    harness.send_key("F5").expect("send F5");
    wait_render();
    harness
        .assert_screen_contains("Audio Server")
        .expect("Should be on Audio Server");

    // F1 → Instrument edit
    harness.send_key("F1").expect("send F1");
    wait_render();
    harness
        .assert_screen_contains("SOURCE:")
        .expect("Should be on instrument edit pane");
}

#[test]
fn test_add_second_instrument() {
    if !require_tmux() {
        eprintln!("tmux not found, skipping test");
        return;
    }

    let harness = start_with_instrument("add-second");

    // Ctrl+g to instrument list, then 'a' to add
    harness.send_key("C-g").expect("send C-g");
    wait_render();
    harness.send_key("a").expect("send a");
    wait_render();
    harness
        .assert_screen_contains("Add Instrument")
        .expect("Ctrl+g then a should open Add Instrument");

    // Add another instrument
    harness.send_key("Enter").expect("send Enter");
    wait_render();

    // Should be back on the Edit pane for the new instrument
    harness
        .assert_screen_contains("Edit:")
        .expect("Should show Edit pane for new instrument");
}
