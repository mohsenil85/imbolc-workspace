//! Imbolc GUI - Cross-platform desktop GUI for Imbolc DAW
//!
//! A Dioxus-based alternative to the terminal UI (imbolc-ui).

mod app;
mod components;
mod dispatch;
mod state;

fn main() {
    env_logger::init();
    log::info!("Starting Imbolc GUI");
    dioxus::launch(app::App);
}
