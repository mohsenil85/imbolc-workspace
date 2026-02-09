//! Network mode functions for server, client, and discovery modes.
//!
//! This module contains all network-related functionality that is only
//! compiled when the "net" feature is enabled.

use std::time::{Duration, Instant};

use imbolc_net::{NetworkState, RemoteDispatcher};
use imbolc_types::Action;

use crate::audio::AudioHandle;
use crate::action::{AudioDirty, IoFeedback};
use crate::config;
use crate::dispatch::LocalDispatcher;
use crate::setup;
use crate::state::{self, AppState};
use crate::ui::{
    Frame, InputSource, LayerStack, RatatuiBackend,
    keybindings,
};
use crate::register_all_panes;

// =============================================================================
// Server Mode
// =============================================================================

pub fn run_server() -> std::io::Result<()> {
    use std::thread;
    use imbolc_net::NetServer;

    log::info!("Starting Imbolc server mode");

    let (io_tx, io_rx) = std::sync::mpsc::channel::<IoFeedback>();
    let config = config::Config::load();
    let state = AppState::new_with_defaults(config.defaults());

    // Create the dispatcher
    let mut dispatcher = LocalDispatcher::new(state, io_tx.clone());

    // Create audio handle and sync initial state
    let mut audio = AudioHandle::new();
    audio.sync_state(dispatcher.state());

    // Auto-start SuperCollider
    let startup_events = setup::auto_start_sc(&mut audio);
    for event in startup_events {
        log::info!("Startup: {:?}", event);
    }

    // Bind server
    let mut server = NetServer::bind("0.0.0.0:9999")?;
    log::info!("Server listening on 0.0.0.0:9999");

    // Register with mDNS for LAN discovery
    #[cfg(feature = "mdns")]
    let _discovery = {
        match imbolc_net::DiscoveryServer::new("Imbolc Session", 9999) {
            Ok(d) => {
                log::info!("mDNS discovery registered");
                Some(d)
            }
            Err(e) => {
                log::warn!("Failed to register mDNS discovery: {}", e);
                None
            }
        }
    };

    let mut pending_audio_dirty = AudioDirty::default();
    let mut last_metering = Instant::now();
    #[cfg(feature = "mdns")]
    let mut last_client_count = 0usize;

    loop {
        // Build network state snapshot
        let network_state = NetworkState {
            session: dispatcher.state().session.clone(),
            instruments: dispatcher.state().instruments.clone(),
            ownership: server.build_ownership_map(),
            privileged_client: server.privileged_client_info(),
        };

        // Accept new connections
        server.accept_connections(&network_state);

        // Heartbeat: ping clients, detect dead connections
        server.tick_heartbeat();

        // Poll for client actions
        for (client_id, net_action) in server.poll_actions(&network_state) {
            log::debug!("Received action from {:?}: {:?}", client_id, net_action);

            // Mark dirty based on action
            server.mark_dirty(&net_action);

            // Convert NetworkAction to Action
            let action = network_action_to_action(net_action);

            // Dispatch
            let result = dispatcher.dispatch_with_audio(&action, &mut audio);
            pending_audio_dirty.merge(result.audio_dirty);

            if result.quit {
                log::info!("Quit requested, shutting down server");
                server.broadcast_shutdown();
                return Ok(());
            }
        }

        // Flush audio dirty flags (always full sync in network server mode)
        if pending_audio_dirty.any() {
            audio.apply_dirty(dispatcher.state(), pending_audio_dirty, true);
            pending_audio_dirty.clear();
        }

        // Broadcast state updates
        let network_state = NetworkState {
            session: dispatcher.state().session.clone(),
            instruments: dispatcher.state().instruments.clone(),
            ownership: server.build_ownership_map(),
            privileged_client: server.privileged_client_info(),
        };
        if server.needs_full_sync() {
            server.broadcast_full_sync(&network_state);
        } else {
            server.broadcast_state_patch(&network_state);
        }

        // Drain I/O feedback (simplified - no UI updates in server mode)
        while let Ok(feedback) = io_rx.try_recv() {
            log::debug!("I/O feedback: {:?}", feedback);
        }

        // Drain audio feedback
        for feedback in audio.drain_feedback() {
            let action = Action::AudioFeedback(feedback);
            let result = dispatcher.dispatch_with_audio(&action, &mut audio);
            pending_audio_dirty.merge(result.audio_dirty);
        }

        // Send metering at ~30Hz
        let now = Instant::now();
        if now.duration_since(last_metering).as_millis() >= 33 {
            last_metering = now;
            let ars = audio.read_state();
            let (peak_l, peak_r) = (audio.master_peak(), audio.master_peak());
            server.broadcast_metering(ars.playhead, ars.bpm, (peak_l, peak_r));

            // Update mDNS client count if changed
            #[cfg(feature = "mdns")]
            {
                let count = server.client_count();
                if count != last_client_count {
                    last_client_count = count;
                    if let Some(ref discovery) = _discovery {
                        discovery.update_client_count(count);
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(2));
    }
}

// =============================================================================
// Discovery Mode (requires mdns feature)
// =============================================================================

/// Discover available Imbolc servers on the LAN and connect to one.
#[cfg(feature = "mdns")]
pub fn run_discovery(own_instruments: Vec<u32>) -> std::io::Result<()> {
    use std::io::{self, Write};
    use imbolc_net::DiscoveryClient;

    println!("Searching for Imbolc servers on LAN...\n");

    // Browse for 3 seconds
    let servers = DiscoveryClient::browse_for(Duration::from_secs(3))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if servers.is_empty() {
        println!("No Imbolc servers found on the local network.");
        println!("\nYou can start a server with: imbolc --server");
        println!("Or connect directly with: imbolc --connect <ip:port>");
        return Ok(());
    }

    println!("Available Imbolc servers on LAN:\n");
    for (i, server) in servers.iter().enumerate() {
        println!(
            "  {}. {}\n     Session: \"{}\" ({} {})\n",
            i + 1,
            server.address,
            server.session_name,
            server.client_count,
            if server.client_count == 1 { "client" } else { "clients" }
        );
    }

    print!("Select server [1-{}] or enter IP address: ", servers.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    // Parse selection
    let addr = if let Ok(num) = input.parse::<usize>() {
        if num >= 1 && num <= servers.len() {
            servers[num - 1].address.clone()
        } else {
            println!("Invalid selection");
            return Ok(());
        }
    } else if !input.is_empty() {
        // Use as direct address
        input.to_string()
    } else {
        println!("No selection made");
        return Ok(());
    };

    println!("\nConnecting to {}...", addr);
    run_client(&addr, own_instruments)
}

// =============================================================================
// Client Mode
// =============================================================================

pub fn run_client(addr: &str, own_instruments: Vec<u32>) -> std::io::Result<()> {
    use crate::ui::action_id::{ActionId, GlobalActionId};

    log::info!("Connecting to server at {}", addr);

    // Get hostname for client name
    let client_name = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Convert CLI instrument IDs to InstrumentId type
    let requested_instruments: Vec<_> = own_instruments.into_iter().collect();

    let mut remote = RemoteDispatcher::connect(addr, &client_name, requested_instruments)?;
    log::info!(
        "Connected to server as {:?}, owning {} instruments",
        remote.client_id(),
        remote.owned_instruments().len()
    );

    // Save session token for reconnection
    if let Err(e) = imbolc_net::save_session(addr, remote.session_token(), &client_name) {
        log::warn!("Failed to save session token: {}", e);
    }

    let mut backend = RatatuiBackend::new()?;
    backend.start()?;

    // Load keybindings
    let (layers, mut keymaps) = keybindings::load_keybindings();

    let mut panes = register_all_panes(&mut keymaps);

    // Create layer stack
    let mut layer_stack = LayerStack::new(layers);
    layer_stack.push("global");

    // Build a synthetic AppState from the network state for rendering
    let config = config::Config::load();
    let mut local_state = AppState::new_with_defaults(config.defaults());
    local_state.session = remote.state().session.clone();
    local_state.instruments = remote.state().instruments.clone();
    sync_network_context(&mut local_state, &remote);

    if local_state.instruments.instruments.is_empty() {
        panes.switch_to("add", &local_state);
    }
    layer_stack.set_pane_layer(panes.active().id());

    let app_frame = Frame::new();
    let mut last_render_time = Instant::now();
    let mut last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    loop {
        // Poll for server updates
        if remote.poll_updates() {
            // State was updated from server
            local_state.session = remote.state().session.clone();
            local_state.instruments = remote.state().instruments.clone();
            sync_network_context(&mut local_state, &remote);
        }

        // Update metering
        let metering = remote.metering();
        local_state.audio.playhead = metering.playhead;
        local_state.audio.bpm = metering.bpm;

        // Check for server shutdown or connection loss
        if remote.server_shutdown() {
            log::info!("Server shut down gracefully, exiting");
            break;
        }
        if remote.connection_lost() {
            log::info!("Connection lost, attempting reconnect...");
            let saved_token = remote.session_token().clone();
            let saved_addr = remote.server_addr().to_string();
            let saved_name = remote.client_name().to_string();

            // Update display to show reconnecting status
            if let Some(ref mut net) = local_state.network {
                net.connection_status = state::NetworkConnectionStatus::Reconnecting;
            }

            // Reconnect with exponential backoff
            let mut delay_ms = 500u64;
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut reconnected = false;

            while Instant::now() < deadline {
                // Render reconnecting state
                let now_render = Instant::now();
                if now_render.duration_since(last_render_time).as_millis() >= 16 {
                    last_render_time = now_render;
                    let mut frame = backend.begin_frame()?;
                    let area = frame.area();
                    last_area = area;
                    let mut rbuf = crate::ui::RenderBuf::new(frame.buffer_mut());
                    app_frame.render_buf(area, &mut rbuf, &local_state);
                    panes.render(area, &mut rbuf, &local_state);
                    backend.end_frame(frame)?;
                }

                std::thread::sleep(Duration::from_millis(delay_ms));

                match RemoteDispatcher::reconnect(&saved_addr, &saved_name, saved_token.clone()) {
                    Ok(new_remote) => {
                        log::info!("Reconnected successfully");
                        remote = new_remote;

                        // Save new session token
                        if let Err(e) = imbolc_net::save_session(&saved_addr, remote.session_token(), &saved_name) {
                            log::warn!("Failed to save session token: {}", e);
                        }

                        // Sync state
                        local_state.session = remote.state().session.clone();
                        local_state.instruments = remote.state().instruments.clone();
                        sync_network_context(&mut local_state, &remote);
                        reconnected = true;
                        break;
                    }
                    Err(e) => {
                        log::warn!("Reconnect attempt failed: {}", e);
                        delay_ms = (delay_ms * 2).min(8000);
                    }
                }
            }

            if !reconnected {
                log::error!("Failed to reconnect within 60s, exiting");
                if let Some(ref mut net) = local_state.network {
                    net.connection_status = state::NetworkConnectionStatus::Disconnected;
                }
                break;
            }
        }

        if let Some(app_event) = backend.poll_event(Duration::from_millis(2)) {
            let pane_action = match app_event {
                crate::ui::AppEvent::Mouse(mouse_event) => {
                    panes.active_mut().handle_mouse(&mouse_event, last_area, &local_state)
                }
                crate::ui::AppEvent::Resize(_, _) => Action::None,
                crate::ui::AppEvent::Key(event) => {
                    match layer_stack.resolve(&event) {
                        crate::ui::LayerResult::Action(action) => {
                            // Handle quit locally
                            if matches!(action, ActionId::Global(GlobalActionId::Quit)) {
                                break;
                            }
                            // Handle privilege request in network mode
                            if matches!(action, ActionId::Global(GlobalActionId::RequestPrivilege)) {
                                if let Err(e) = remote.request_privilege() {
                                    log::warn!("Failed to request privilege: {}", e);
                                }
                                Action::None
                            } else {
                                panes.active_mut().handle_action(action, &event, &local_state)
                            }
                        }
                        crate::ui::LayerResult::Blocked | crate::ui::LayerResult::Unresolved => {
                            panes.active_mut().handle_raw_input(&event, &local_state)
                        }
                    }
                }
            };

            // Handle layer management locally
            match &pane_action {
                Action::PushLayer(name) => layer_stack.push(name),
                Action::PopLayer(name) => layer_stack.pop(name),
                Action::ExitPerformanceMode => {
                    layer_stack.pop("piano_mode");
                    layer_stack.pop("pad_mode");
                    panes.active_mut().deactivate_performance();
                }
                _ => {}
            }

            // Navigation handled locally
            panes.process_nav(&pane_action, &local_state);
            if matches!(&pane_action, Action::Nav(_)) {
                layer_stack.set_pane_layer(panes.active().id());
            }

            // Convert to NetworkAction and send to server
            if let Some(net_action) = action_to_network_action(&pane_action) {
                if let Err(e) = remote.dispatch(net_action) {
                    log::error!("Failed to send action to server: {}", e);
                    break;
                }
            }

            // Local quit
            if matches!(&pane_action, Action::Quit) {
                break;
            }
        }

        // Render at ~60fps
        let now_render = Instant::now();
        if now_render.duration_since(last_render_time).as_millis() >= 16 {
            last_render_time = now_render;

            let mut frame = backend.begin_frame()?;
            let area = frame.area();
            last_area = area;
            let mut rbuf = crate::ui::RenderBuf::new(frame.buffer_mut());
            app_frame.render_buf(area, &mut rbuf, &local_state);
            panes.render(area, &mut rbuf, &local_state);
            backend.end_frame(frame)?;
        }
    }

    let _ = remote.disconnect();
    imbolc_net::clear_session();
    backend.stop()?;
    Ok(())
}

// =============================================================================
// Action Conversion Utilities
// =============================================================================

/// Convert NetworkAction to Action for dispatch.
pub fn network_action_to_action(net_action: imbolc_net::NetworkAction) -> Action {
    use imbolc_net::NetworkAction;
    match net_action {
        NetworkAction::None => Action::None,
        NetworkAction::Quit => Action::Quit,
        NetworkAction::Instrument(a) => Action::Instrument(a),
        NetworkAction::Mixer(a) => Action::Mixer(a),
        NetworkAction::PianoRoll(a) => Action::PianoRoll(a),
        NetworkAction::Arrangement(a) => Action::Arrangement(a),
        NetworkAction::Server(a) => Action::Server(a),
        NetworkAction::Session(a) => Action::Session(a),
        NetworkAction::Sequencer(a) => Action::Sequencer(a),
        NetworkAction::Chopper(a) => Action::Chopper(a),
        NetworkAction::Automation(a) => Action::Automation(a),
        NetworkAction::Midi(a) => Action::Midi(a),
        NetworkAction::Bus(a) => Action::Bus(a),
        NetworkAction::LayerGroup(a) => Action::LayerGroup(a),
        NetworkAction::VstParam(a) => Action::VstParam(a),
        NetworkAction::Undo => Action::Undo,
        NetworkAction::Redo => Action::Redo,
    }
}

/// Convert Action to NetworkAction for transmission (returns None for local-only actions).
pub fn action_to_network_action(action: &Action) -> Option<imbolc_net::NetworkAction> {
    use imbolc_net::NetworkAction;
    match action {
        Action::None => Some(NetworkAction::None),
        Action::Quit => Some(NetworkAction::Quit),
        Action::Instrument(a) => Some(NetworkAction::Instrument(a.clone())),
        Action::Mixer(a) => Some(NetworkAction::Mixer(a.clone())),
        Action::PianoRoll(a) => Some(NetworkAction::PianoRoll(a.clone())),
        Action::Arrangement(a) => Some(NetworkAction::Arrangement(a.clone())),
        Action::Server(a) => Some(NetworkAction::Server(a.clone())),
        Action::Session(a) => Some(NetworkAction::Session(a.clone())),
        Action::Sequencer(a) => Some(NetworkAction::Sequencer(a.clone())),
        Action::Chopper(a) => Some(NetworkAction::Chopper(a.clone())),
        Action::Automation(a) => Some(NetworkAction::Automation(a.clone())),
        Action::Midi(a) => Some(NetworkAction::Midi(a.clone())),
        Action::Bus(a) => Some(NetworkAction::Bus(a.clone())),
        Action::LayerGroup(a) => Some(NetworkAction::LayerGroup(a.clone())),
        Action::VstParam(a) => Some(NetworkAction::VstParam(a.clone())),
        Action::Undo => Some(NetworkAction::Undo),
        Action::Redo => Some(NetworkAction::Redo),
        // Local-only actions
        Action::Nav(_) => None,
        Action::AudioFeedback(_) => None,
        Action::ExitPerformanceMode => None,
        Action::PushLayer(_) => None,
        Action::PopLayer(_) => None,
        Action::SaveAndQuit => None,
        Action::Click(_) => None,
    }
}

/// Sync network display context from RemoteDispatcher to AppState.
pub fn sync_network_context(local_state: &mut AppState, remote: &RemoteDispatcher) {
    use std::collections::HashMap;
    use imbolc_net::OwnershipStatus;
    use state::{ClientDisplayInfo, NetworkConnectionStatus, NetworkDisplayContext, OwnershipDisplayStatus};

    let mut ownership = HashMap::new();

    for inst in &local_state.instruments.instruments {
        let status = match remote.ownership_status(inst.id) {
            OwnershipStatus::OwnedByMe => OwnershipDisplayStatus::OwnedByMe,
            OwnershipStatus::OwnedByOther(name) => OwnershipDisplayStatus::OwnedByOther(name),
            OwnershipStatus::Unowned => OwnershipDisplayStatus::Unowned,
        };
        ownership.insert(inst.id, status);
    }

    let privileged_client_name = remote.privileged_client().map(|(_, name)| name.to_string());

    // Build connected clients list by deduplicating the ownership map by client name
    let mut client_counts: HashMap<String, (bool, usize)> = HashMap::new();
    for owner_info in remote.ownership_map().values() {
        let is_priv = remote.privileged_client()
            .map(|(id, _)| id == owner_info.client_id)
            .unwrap_or(false);
        let entry = client_counts.entry(owner_info.client_name.clone()).or_insert((is_priv, 0));
        entry.1 += 1;
    }
    // Always include ourselves
    let client_name = remote.client_name().to_string();
    client_counts.entry(client_name.clone()).or_insert((remote.is_privileged(), 0));

    let connected_clients: Vec<ClientDisplayInfo> = client_counts
        .into_iter()
        .map(|(name, (is_privileged, owned_instrument_count))| ClientDisplayInfo {
            name,
            is_privileged,
            owned_instrument_count,
        })
        .collect();

    let connection_status = if remote.connection_lost() {
        NetworkConnectionStatus::Disconnected
    } else {
        NetworkConnectionStatus::Connected
    };

    local_state.network = Some(NetworkDisplayContext {
        ownership,
        is_privileged: remote.is_privileged(),
        privileged_client_name,
        connection_status,
        client_name,
        connected_clients,
    });
}
