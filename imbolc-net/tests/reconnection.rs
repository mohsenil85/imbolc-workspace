mod common;

use imbolc_net::protocol::{ClientMessage, PrivilegeLevel, ServerMessage, SessionToken};
use imbolc_net::server::NetServer;
use imbolc_types::InstrumentId;
use std::time::Duration;

#[test]
fn test_graceful_disconnect_suspends_session() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 3);

    // Alice connects with instruments 0, 1
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice
        .send_hello(
            "Alice",
            vec![InstrumentId::new(0), InstrumentId::new(1)],
            false,
        )
        .unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let welcome = alice.recv().unwrap();
    let _token = match welcome {
        ServerMessage::Welcome { session_token, .. } => session_token,
        other => panic!("Expected Welcome, got {:?}", other),
    };

    assert_eq!(server.client_count(), 1);

    // Alice disconnects gracefully
    alice.send(&ClientMessage::Goodbye).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let state = common::make_test_state_with_instruments(&server, 3);
    server.accept_connections();
    server.poll_actions(&state.session, &state.instruments);

    // Client should be suspended (count drops)
    assert_eq!(server.client_count(), 0);
}

#[test]
fn test_reconnect_with_valid_token() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 3);

    // Alice connects with instruments 0, 1 and privilege
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice
        .send_hello(
            "Alice",
            vec![InstrumentId::new(0), InstrumentId::new(1)],
            true,
        )
        .unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let welcome = alice.recv().unwrap();
    let token = match welcome {
        ServerMessage::Welcome {
            session_token,
            granted_instruments,
            privilege,
            ..
        } => {
            assert_eq!(granted_instruments.len(), 2);
            assert_eq!(privilege, PrivilegeLevel::Privileged);
            session_token
        }
        other => panic!("Expected Welcome, got {:?}", other),
    };

    // Alice disconnects
    alice.send(&ClientMessage::Goodbye).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let state = common::make_test_state_with_instruments(&server, 3);
    server.accept_connections();
    server.poll_actions(&state.session, &state.instruments);
    assert_eq!(server.client_count(), 0);

    // Alice reconnects with token
    let state = common::make_test_state_with_instruments(&server, 3);
    let mut alice2 = common::RawClient::connect(&addr).unwrap();
    alice2.send_reconnect("Alice", token).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let reconnect_msg = alice2.recv().unwrap();
    match reconnect_msg {
        ServerMessage::ReconnectSuccessful {
            restored_instruments,
            privilege,
            ..
        } => {
            assert_eq!(restored_instruments.len(), 2);
            assert!(restored_instruments.contains(&InstrumentId::new(0)));
            assert!(restored_instruments.contains(&InstrumentId::new(1)));
            assert_eq!(privilege, PrivilegeLevel::Privileged);
        }
        other => panic!("Expected ReconnectSuccessful, got {:?}", other),
    }

    // Server should send StateUpdate immediately after ReconnectSuccessful
    let state_msg = alice2.recv().unwrap();
    match state_msg {
        ServerMessage::StateUpdate { state } => {
            // Verify the state has the expected instruments
            assert_eq!(state.instruments.instruments.len(), 3);
        }
        other => panic!("Expected StateUpdate after reconnect, got {:?}", other),
    }

    assert_eq!(server.client_count(), 1);
}

#[test]
fn test_reconnect_with_invalid_token_fails() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Alice connects
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Alice disconnects
    alice.send(&ClientMessage::Goodbye).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let state = common::make_test_state(&server);
    server.accept_connections();
    server.poll_actions(&state.session, &state.instruments);

    // Try to reconnect with a fake token
    let state = common::make_test_state(&server);
    let mut alice2 = common::RawClient::connect(&addr).unwrap();
    let fake_token = SessionToken("fake-token-12345".into());
    alice2.send_reconnect("Alice", fake_token).unwrap();

    // Drive server until it processes the failed reconnect.
    // The client won't be promoted, so we can't use drive_until_clients.
    // Instead, loop until pending_count drops back to 0 (processed but rejected).
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        server.accept_connections();
        server.poll_actions(&state.session, &state.instruments);
        if server.pending_count() == 0 && server.client_count() == 0 {
            // Give a tiny extra moment for the message to be flushed
            std::thread::sleep(Duration::from_millis(10));
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let msg = alice2.recv().unwrap();
    match msg {
        ServerMessage::ReconnectFailed { reason } => {
            assert!(
                reason.contains("expired") || reason.contains("invalid"),
                "Reason should mention expiry or invalid: {}",
                reason
            );
        }
        other => panic!("Expected ReconnectFailed, got {:?}", other),
    }
}
