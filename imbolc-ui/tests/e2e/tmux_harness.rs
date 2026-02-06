use std::process::Command;
use std::thread;
use std::time::Duration;

const WIDTH: u32 = 120;
const HEIGHT: u32 = 40;

pub struct TmuxHarness {
    session_name: String,
}

impl TmuxHarness {
    pub fn new(test_name: &str) -> Self {
        let session_name = format!(
            "imbolc-{}-{}",
            test_name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        Self { session_name }
    }

    /// Start the app in a new tmux session
    pub fn start(&self, command: &str) -> Result<(), String> {
        // Create new detached tmux session running the command.
        // Env vars must be set via set-environment (not .env()) because the
        // tmux server spawns child processes in its own environment.
        let wrapped = format!("IMBOLC_NO_AUDIO=1 {}", command);
        let status = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &self.session_name,
                "-x",
                &WIDTH.to_string(),
                "-y",
                &HEIGHT.to_string(),
                &wrapped,
            ])
            .env_remove("TMUX") // Allow nested tmux
            .status()
            .map_err(|e| format!("Failed to start tmux: {}", e))?;

        if !status.success() {
            return Err("tmux new-session failed".to_string());
        }

        // Wait for app to start
        thread::sleep(Duration::from_millis(1000));
        Ok(())
    }

    /// Send keys to the tmux session
    pub fn send_keys(&self, keys: &[&str]) -> Result<(), String> {
        for key in keys {
            let status = Command::new("tmux")
                .args(["send-keys", "-t", &self.session_name, key])
                .env_remove("TMUX")
                .status()
                .map_err(|e| format!("Failed to send keys: {}", e))?;

            if !status.success() {
                return Err(format!("tmux send-keys failed for key: {}", key));
            }
            thread::sleep(Duration::from_millis(50));
        }
        thread::sleep(Duration::from_millis(100));
        Ok(())
    }

    /// Send a single key
    pub fn send_key(&self, key: &str) -> Result<(), String> {
        self.send_keys(&[key])
    }

    /// Capture the current screen content
    pub fn capture_screen(&self) -> Result<String, String> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-t", &self.session_name, "-p"])
            .env_remove("TMUX")
            .output()
            .map_err(|e| format!("Failed to capture screen: {}", e))?;

        if !output.status.success() {
            return Err("tmux capture-pane failed".to_string());
        }

        String::from_utf8(output.stdout)
            .map_err(|e| format!("Invalid UTF-8 in screen capture: {}", e))
    }

    /// Assert the screen contains specific text
    pub fn assert_screen_contains(&self, text: &str) -> Result<(), String> {
        let screen = self.capture_screen()?;
        if screen.contains(text) {
            Ok(())
        } else {
            Err(format!(
                "Expected screen to contain: '{}'\nActual screen:\n{}",
                text, screen
            ))
        }
    }

    /// Check if the tmux session is still running
    pub fn is_running(&self) -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", &self.session_name])
            .env_remove("TMUX")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Wait for the session to exit (with timeout)
    pub fn wait_for_exit(&self, timeout: Duration) -> Result<(), String> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if !self.is_running() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Timeout waiting for session to exit".to_string())
    }

    /// Kill the tmux session
    pub fn kill(&self) {
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", &self.session_name])
            .env_remove("TMUX")
            .status();
    }
}

impl Drop for TmuxHarness {
    fn drop(&mut self) {
        self.kill();
    }
}
