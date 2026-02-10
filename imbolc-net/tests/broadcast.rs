mod common;

use std::time::{Duration, Instant};
use imbolc_net::server::NetServer;
use imbolc_net::protocol::{NetworkAction, ServerMessage};
use imbolc_types::{
    ArrangementAction, AutomationAction, AutomationTarget, BusAction, InstrumentAction,
    InstrumentParameter, MixerAction, ParameterTarget, PianoRollAction, ServerAction, SourceType,
    VstParamAction, VstTarget,
};

#[test]
fn test_metering_broadcast() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Broadcast metering
    server.broadcast_metering(120, 128.0, (0.75, 0.65));

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::Metering { playhead, bpm, peaks } => {
            assert_eq!(playhead, 120);
            assert!((bpm - 128.0).abs() < 0.01);
            assert!((peaks.0 - 0.75).abs() < 0.01);
            assert!((peaks.1 - 0.65).abs() < 0.01);
        }
        other => panic!("Expected Metering, got {:?}", other),
    }
}

#[test]
fn test_shutdown_broadcast() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    let state = common::make_test_state(&server);

    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello("Bob", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));
    let _bob_welcome = bob.recv().unwrap();

    // Broadcast shutdown
    server.broadcast_shutdown();

    let alice_msg = alice.recv().unwrap();
    match alice_msg {
        ServerMessage::Shutdown => {}
        other => panic!("Expected Shutdown for Alice, got {:?}", other),
    }

    let bob_msg = bob.recv().unwrap();
    match bob_msg {
        ServerMessage::Shutdown => {}
        other => panic!("Expected Shutdown for Bob, got {:?}", other),
    }
}

#[test]
fn test_state_patch_broadcast() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Mark session dirty and broadcast patch
    use imbolc_net::protocol::NetworkAction;
    use imbolc_types::ServerAction;
    server.mark_dirty(&NetworkAction::Server(ServerAction::RecordMaster));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            // Session should be present (Server action marks session dirty)
            assert!(patch.session.is_some(), "Session should be in the patch");
            // Instruments should NOT be present
            assert!(patch.instruments.is_none(), "Instruments should not be in the patch");
            assert!(patch.seq > 0);
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_full_sync_broadcast() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    let state = common::make_test_state(&server);
    server.broadcast_full_sync(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::FullStateSync { seq, .. } => {
            assert!(seq > 0);
        }
        other => panic!("Expected FullStateSync, got {:?}", other),
    }
}

// ── Patch correctness: instruments-only ─────────────────────────

#[test]
fn test_patch_instruments_only() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.instruments.is_some(), "instruments should be present");
            assert!(patch.session.is_none(), "session should be absent");
            assert!(patch.ownership.is_none(), "ownership should be absent");
            assert!(patch.privileged_client.is_none(), "privileged_client should be absent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── Patch correctness: mixer + instruments ──────────────────────

#[test]
fn test_patch_mixer_and_instruments() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Mixer marks mixer (granular) + instruments_structural
    server.mark_dirty(&NetworkAction::Mixer(MixerAction::Move(1)));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_none(), "session should be absent (Mixer is granular)");
            assert!(patch.mixer.is_some(), "mixer should be present");
            assert!(patch.instruments.is_some(), "instruments should be present");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── No broadcast when nothing is dirty ──────────────────────────

#[test]
fn test_patch_no_broadcast_when_clean() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Don't mark anything dirty — broadcast_state_patch should be a no-op
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    // Set a short read timeout to confirm nothing arrives
    alice.reader.get_ref().set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let result = alice.recv();
    assert!(result.is_err(), "Should not receive anything when nothing is dirty");
}

// ── Sequence numbers increment across patches ───────────────────

#[test]
fn test_seq_increments() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    let mut prev_seq = 0u64;
    for _ in 0..3 {
        server.mark_dirty(&NetworkAction::Server(ServerAction::Connect));
        server.reset_rate_limit();
        let state = common::make_test_state(&server);
        server.broadcast_state_patch(&state);

        let msg = alice.recv().unwrap();
        match msg {
            ServerMessage::StatePatchUpdate { patch } => {
                assert!(patch.seq > prev_seq, "seq should increase: {} > {}", patch.seq, prev_seq);
                prev_seq = patch.seq;
            }
            other => panic!("Expected StatePatchUpdate, got {:?}", other),
        }
    }
}

// ── Dirty flags clear after broadcast ───────────────────────────

#[test]
fn test_dirty_clears_after_patch() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // First broadcast: instruments dirty
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);
    let _msg1 = alice.recv().unwrap();

    // Second broadcast: session dirty (instruments should be clean now)
    server.mark_dirty(&NetworkAction::Server(ServerAction::Connect));
    server.reset_rate_limit();
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg2 = alice.recv().unwrap();
    match msg2 {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_some(), "session should be present");
            assert!(patch.instruments.is_none(), "instruments should be cleared from previous broadcast");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── Ownership patch via mark_ownership_dirty ────────────────────

#[test]
fn test_ownership_patch() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // mark_ownership_dirty sets both ownership + privileged_client
    server.mark_ownership_dirty();
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.ownership.is_some(), "ownership should be present");
            // Some(None) = "changed to: nobody is privileged" survives JSON
            // thanks to the double_option serde helper (field present as null).
            assert_eq!(patch.privileged_client, Some(None),
                "Some(None) should survive JSON roundtrip");
            assert!(patch.session.is_none(), "session should be absent");
            assert!(patch.instruments.is_none(), "instruments should be absent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── Privileged client patch survives JSON when Some(Some(...)) ──

#[test]
fn test_privileged_client_patch_with_holder() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Alice requests privilege
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    let state = common::make_test_state(&server);
    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello("Bob", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));
    let _welcome_b = bob.recv().unwrap();

    // Now mark ownership dirty — privileged_client is Some(Some(...)) so it survives JSON
    server.mark_ownership_dirty();
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = bob.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.ownership.is_some(), "ownership should be present");
            assert!(patch.privileged_client.is_some(), "privileged_client should survive JSON when holder exists");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── Multiple clients receive same patch ─────────────────────────

#[test]
fn test_patch_reaches_all_clients() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome_a = alice.recv().unwrap();

    let state = common::make_test_state(&server);
    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello("Bob", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));
    let _welcome_b = bob.recv().unwrap();

    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    for (name, client) in [("Alice", &mut alice), ("Bob", &mut bob)] {
        let msg = client.recv().unwrap();
        match msg {
            ServerMessage::StatePatchUpdate { patch } => {
                assert!(patch.instruments.is_some(), "{} should receive instruments", name);
            }
            other => panic!("{}: Expected StatePatchUpdate, got {:?}", name, other),
        }
    }
}

// ── Accumulated actions produce combined patch ──────────────────

#[test]
fn test_accumulated_actions_combined_patch() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Mark session dirty then instruments dirty before broadcasting
    server.mark_dirty(&NetworkAction::Server(ServerAction::Connect));
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));
    server.mark_ownership_dirty();

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_some(), "session should be present");
            assert!(patch.instruments.is_some(), "instruments should be present");
            assert!(patch.ownership.is_some(), "ownership should be present");
            assert!(patch.privileged_client.is_some(), "privileged_client should be present");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── Per-instrument delta patches ────────────────────────────────

#[test]
fn test_patch_single_instrument_change() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();

    // State with 4 instruments so targeted threshold isn't hit
    let state = common::make_test_state_with_instruments(&server, 4);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Targeted action on instrument 1
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(1, 0.1)));
    let state = common::make_test_state_with_instruments(&server, 4);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.instruments.is_none(), "full instruments should NOT be sent for targeted change");
            let patches = patch.instrument_patches.expect("instrument_patches should be present");
            assert!(patches.contains_key(&1), "instrument 1 should be in patches");
            assert_eq!(patches.len(), 1);
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_patch_structural_sends_full_instruments() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 4);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Structural action (Add)
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));
    let state = common::make_test_state_with_instruments(&server, 4);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.instruments.is_some(), "full instruments should be sent for structural");
            assert!(patch.instrument_patches.is_none(), "instrument_patches should be absent for structural");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_patch_targeted_then_structural() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 4);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Targeted + structural in same tick → structural wins
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(1, 0.1)));
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::Add(SourceType::Saw)));
    let state = common::make_test_state_with_instruments(&server, 4);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.instruments.is_some(), "structural should override to full instruments");
            assert!(patch.instrument_patches.is_none(), "instrument_patches absent when full instruments sent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_instrument_patches_roundtrip() {
    // Verify instrument_patches survive JSON serialization over the wire
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 4);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Two targeted changes on different instruments
    server.mark_dirty(&NetworkAction::VstParam(VstParamAction::SetParam(0, VstTarget::Source, 0, 0.5)));
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(2, 0.3)));
    let state = common::make_test_state_with_instruments(&server, 4);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            let patches = patch.instrument_patches.expect("instrument_patches should be present");
            assert!(patches.contains_key(&0), "instrument 0 should be in patches");
            assert!(patches.contains_key(&2), "instrument 2 should be in patches");
            assert_eq!(patches.len(), 2);
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_patch_rate_limiting() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // First broadcast should go through
    server.mark_dirty(&NetworkAction::Server(ServerAction::Connect));
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);
    let msg = alice.recv().unwrap();
    assert!(matches!(msg, ServerMessage::StatePatchUpdate { .. }));

    // Second broadcast immediately after should be rate-limited (no reset_rate_limit)
    server.mark_dirty(&NetworkAction::Server(ServerAction::RecordMaster));
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    // Should not receive anything (rate-limited)
    alice.reader.get_ref().set_read_timeout(Some(Duration::from_millis(200))).unwrap();
    let result = alice.recv();
    assert!(result.is_err(), "Second broadcast should be rate-limited");

    // After reset, the accumulated dirty flags should produce a broadcast
    server.reset_rate_limit();
    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    alice.reader.get_ref().set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_some(), "accumulated session should be present after rate-limit passes");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_patch_threshold_coalescing() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    // 4 instruments, threshold is >2
    let state = common::make_test_state_with_instruments(&server, 4);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Dirty 3 out of 4 instruments (> half) → should coalesce to full instruments
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(0, 0.1)));
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(1, 0.2)));
    server.mark_dirty(&NetworkAction::Instrument(InstrumentAction::AdjustFilterCutoff(2, 0.3)));
    let state = common::make_test_state_with_instruments(&server, 4);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(
                patch.instruments.is_some(),
                "should coalesce to full instruments when >half are dirty"
            );
            assert!(
                patch.instrument_patches.is_none(),
                "instrument_patches should be absent when coalesced"
            );
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

// ── Slow-client mitigation ──────────────────────────────────────

#[test]
fn test_slow_client_does_not_block_fast_client() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Alice reads promptly
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome_a = alice.recv().unwrap();

    // Bob connects but will NOT read — his TCP buffer will fill up
    let state = common::make_test_state(&server);
    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.writer.get_ref().set_nodelay(true).unwrap();
    bob.send_hello("Bob", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));
    // Don't read Bob's welcome — let his buffer fill

    // Broadcast many messages — Alice should receive them even if Bob is slow
    let start = Instant::now();
    let mut alice_received = 0u32;
    for i in 0..20 {
        server.mark_dirty(&NetworkAction::Server(ServerAction::Connect));
        server.reset_rate_limit();
        let state = common::make_test_state(&server);
        server.broadcast_state_patch(&state);
        server.flush_outboxes();

        // Try to read from Alice with a short timeout
        alice.reader.get_ref().set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        if let Ok(msg) = alice.recv() {
            if matches!(msg, ServerMessage::StatePatchUpdate { .. }) {
                alice_received += 1;
            }
        }

        // Check we haven't been blocked for too long
        if start.elapsed() > Duration::from_secs(10) {
            panic!("Server loop took too long — slow client is blocking (iteration {})", i);
        }
    }

    // Alice should have received most/all messages
    assert!(alice_received >= 10,
        "Alice should receive most broadcasts (got {}/20), slow Bob should not block her",
        alice_received);
}

#[test]
fn test_stalled_client_suspended_via_outbox_overflow() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Connect a client normally
    let mut client = common::RawClient::connect(&addr).unwrap();
    client.send_hello("Stalled", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = client.recv().unwrap();
    assert_eq!(server.client_count(), 1);

    // Drop the client side — this will cause writes to eventually fail with BrokenPipe.
    // We need a small delay for the TCP RST to propagate.
    drop(client);
    std::thread::sleep(Duration::from_millis(50));

    // Inject frames into the outbox. When flush_outboxes tries to write them,
    // the broken connection will cause an error, triggering suspension.
    server.inject_outbox_frames(10);
    server.flush_outboxes();

    assert_eq!(server.client_count(), 0,
        "Client with broken connection should be suspended after flush_outboxes");
}

#[test]
fn test_broken_client_detected_on_broadcast() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Connect a client normally
    let mut client = common::RawClient::connect(&addr).unwrap();
    client.send_hello("Stalled", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = client.recv().unwrap();
    assert_eq!(server.client_count(), 1);

    // Drop the client and wait for TCP RST to propagate
    drop(client);
    std::thread::sleep(Duration::from_millis(100));

    // The first write may succeed (buffered by kernel). Keep broadcasting
    // until the broken pipe is detected — should happen within a few writes.
    for _ in 0..10 {
        server.broadcast_shutdown();
        if server.client_count() == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(server.client_count(), 0,
        "Client with broken connection should be suspended after repeated writes");
}

// ── Subsystem-level session patch granularity ───────────────────

#[test]
fn test_piano_roll_only_sends_piano_roll() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    server.mark_dirty(&NetworkAction::PianoRoll(PianoRollAction::PlayStop));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.piano_roll.is_some(), "piano_roll should be present");
            assert!(patch.session.is_none(), "session should be absent");
            assert!(patch.arrangement.is_none(), "arrangement should be absent");
            assert!(patch.automation.is_none(), "automation should be absent");
            assert!(patch.mixer.is_none(), "mixer should be absent");
            assert!(patch.instruments.is_none(), "instruments should be absent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_arrangement_only_sends_arrangement() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    server.mark_dirty(&NetworkAction::Arrangement(ArrangementAction::TogglePlayMode));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.arrangement.is_some(), "arrangement should be present");
            assert!(patch.session.is_none(), "session should be absent");
            assert!(patch.piano_roll.is_none(), "piano_roll should be absent");
            assert!(patch.automation.is_none(), "automation should be absent");
            assert!(patch.mixer.is_none(), "mixer should be absent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_mixer_only_sends_mixer() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    server.mark_dirty(&NetworkAction::Bus(BusAction::Add));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.mixer.is_some(), "mixer should be present");
            assert!(patch.session.is_none(), "session should be absent");
            assert!(patch.piano_roll.is_none(), "piano_roll should be absent");
            assert!(patch.arrangement.is_none(), "arrangement should be absent");
            assert!(patch.automation.is_none(), "automation should be absent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_undo_sends_full_session() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    server.mark_dirty(&NetworkAction::Undo);

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_some(), "session should be present (undo sends full)");
            // When full session is sent, granular fields should be absent
            assert!(patch.piano_roll.is_none(), "piano_roll absent when full session");
            assert!(patch.arrangement.is_none(), "arrangement absent when full session");
            assert!(patch.automation.is_none(), "automation absent when full session");
            assert!(patch.mixer.is_none(), "mixer absent when full session");
            assert!(patch.instruments.is_some(), "instruments should be present (undo is structural)");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}

#[test]
fn test_mixed_subsystems_no_full_session() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Mark both PianoRoll and Automation dirty
    server.mark_dirty(&NetworkAction::PianoRoll(PianoRollAction::PlayStop));
    server.mark_dirty(&NetworkAction::Automation(AutomationAction::AddLane(
        AutomationTarget::Instrument(0, InstrumentParameter::Standard(ParameterTarget::Level)),
    )));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.piano_roll.is_some(), "piano_roll should be present");
            assert!(patch.automation.is_some(), "automation should be present");
            assert!(patch.session.is_none(), "session should be absent (no remainder change)");
            assert!(patch.arrangement.is_none(), "arrangement should be absent");
            assert!(patch.mixer.is_none(), "mixer should be absent");
        }
        other => panic!("Expected StatePatchUpdate, got {:?}", other),
    }
}
