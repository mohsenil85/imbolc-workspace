mod common;

use std::time::Duration;
use imbolc_net::server::NetServer;
use imbolc_net::protocol::ServerMessage;
use imbolc_types::InstrumentId;

#[test]
fn test_contested_ownership() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 3);

    // Alice requests instruments 0, 1
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![InstrumentId::new(0), InstrumentId::new(1)], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let alice_welcome = alice.recv().unwrap();
    match alice_welcome {
        ServerMessage::Welcome { granted_instruments, .. } => {
            assert_eq!(granted_instruments.len(), 2);
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }

    // Rebuild state after Alice connects
    let state = common::make_test_state_with_instruments(&server, 3);

    // Bob requests instruments 1, 2 — should only get 2 (1 is taken)
    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello("Bob", vec![InstrumentId::new(1), InstrumentId::new(2)], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));

    let bob_welcome = bob.recv().unwrap();
    match bob_welcome {
        ServerMessage::Welcome { granted_instruments, .. } => {
            assert_eq!(granted_instruments.len(), 1);
            assert!(granted_instruments.contains(&InstrumentId::new(2)));
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }
}
