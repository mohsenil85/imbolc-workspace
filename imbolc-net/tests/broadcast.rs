mod common;

use std::time::Duration;
use imbolc_net::server::NetServer;
use imbolc_net::protocol::{NetworkAction, ServerMessage};
use imbolc_types::{InstrumentAction, MixerAction, ServerAction, SourceType};

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

// ── Patch correctness: both session + instruments ───────────────

#[test]
fn test_patch_session_and_instruments() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Mixer marks both session + instruments
    server.mark_dirty(&NetworkAction::Mixer(MixerAction::Move(1)));

    let state = common::make_test_state(&server);
    server.broadcast_state_patch(&state);

    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_some(), "session should be present");
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
