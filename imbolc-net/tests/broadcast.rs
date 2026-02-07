mod common;

use std::time::Duration;
use imbolc_net::server::NetServer;
use imbolc_net::protocol::ServerMessage;

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
