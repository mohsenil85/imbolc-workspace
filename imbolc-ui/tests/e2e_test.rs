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
// Tests (examples only; ignored by default)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
