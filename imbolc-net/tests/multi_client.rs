mod common;

use imbolc_net::protocol::ServerMessage;
use imbolc_net::server::NetServer;
use imbolc_types::InstrumentId;
use std::time::Duration;

#[test]
fn test_two_clients_independent_ownership() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 4);

    // Alice owns 0, 1
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice
        .send_hello(
            "Alice",
            vec![InstrumentId::new(0), InstrumentId::new(1)],
            false,
        )
        .unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let alice_welcome = alice.recv().unwrap();
    match alice_welcome {
        ServerMessage::Welcome {
            granted_instruments,
            ..
        } => {
            assert_eq!(granted_instruments.len(), 2);
        }
        _ => panic!("Expected Welcome"),
    }

    let state = common::make_test_state_with_instruments(&server, 4);

    // Bob owns 2, 3
    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello(
        "Bob",
        vec![InstrumentId::new(2), InstrumentId::new(3)],
        false,
    )
    .unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));

    let bob_welcome = bob.recv().unwrap();
    match bob_welcome {
        ServerMessage::Welcome {
            granted_instruments,
            ..
        } => {
            assert_eq!(granted_instruments.len(), 2);
        }
        _ => panic!("Expected Welcome"),
    }

    assert_eq!(server.client_count(), 2);
}

#[test]
fn test_state_broadcast_to_all() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _alice_welcome = alice.recv().unwrap();

    let state = common::make_test_state(&server);

    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello("Bob", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));
    let _bob_welcome = bob.recv().unwrap();

    // Broadcast state
    let state = common::make_test_state(&server);
    server.broadcast_state(&state);
    server.flush_writer();

    // Both clients should receive the state update
    let alice_msg = alice.recv().unwrap();
    match alice_msg {
        ServerMessage::StateUpdate { .. } => {}
        other => panic!("Expected StateUpdate for Alice, got {:?}", other),
    }

    let bob_msg = bob.recv().unwrap();
    match bob_msg {
        ServerMessage::StateUpdate { .. } => {}
        other => panic!("Expected StateUpdate for Bob, got {:?}", other),
    }
}
