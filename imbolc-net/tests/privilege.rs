mod common;

use std::time::Duration;
use imbolc_net::server::NetServer;
use imbolc_net::protocol::{ClientMessage, NetworkAction, PrivilegeLevel, ServerMessage};
use imbolc_types::ServerAction;

#[test]
fn test_reject_unprivileged_transport() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Alice connects without privilege
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));
    let _welcome = alice.recv().unwrap();

    // Alice sends a server action (requires privilege)
    alice.send(&ClientMessage::Action(
        NetworkAction::Server(ServerAction::RecordMaster),
    )).unwrap();

    // Drive server to process the action
    std::thread::sleep(Duration::from_millis(50));
    let state = common::make_test_state(&server);
    let actions = common::drive_and_collect_actions(&mut server, &state, Duration::from_millis(200));
    assert!(actions.is_empty(), "Unprivileged server action should be rejected");

    // Wait for writer thread to deliver the rejection
    server.flush_writer();

    // Alice should receive an ActionRejected message
    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::ActionRejected { reason } => {
            assert!(reason.contains("privilege"), "Rejection reason should mention privilege: {}", reason);
        }
        other => panic!("Expected ActionRejected, got {:?}", other),
    }
}

#[test]
fn test_privilege_transfer() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    // Alice connects with privilege
    let mut alice = common::RawClient::connect(&addr).unwrap();
    alice.send_hello("Alice", vec![], true).unwrap();
    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let alice_welcome = alice.recv().unwrap();
    match alice_welcome {
        ServerMessage::Welcome { privilege, .. } => {
            assert_eq!(privilege, PrivilegeLevel::Privileged);
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }

    let state = common::make_test_state(&server);

    // Bob connects without privilege
    let mut bob = common::RawClient::connect(&addr).unwrap();
    bob.send_hello("Bob", vec![], false).unwrap();
    common::drive_until_clients(&mut server, &state, 2, Duration::from_secs(2));
    let _bob_welcome = bob.recv().unwrap();

    // Bob requests privilege
    bob.send(&ClientMessage::RequestPrivilege).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let state = common::make_test_state(&server);
    server.accept_connections();
    server.poll_actions(&state.session, &state.instruments);
    server.flush_writer();

    // Bob should receive PrivilegeGranted
    let msg = bob.recv().unwrap();
    match msg {
        ServerMessage::PrivilegeGranted => {}
        other => panic!("Expected PrivilegeGranted, got {:?}", other),
    }

    // Alice should receive PrivilegeRevoked
    let msg = alice.recv().unwrap();
    match msg {
        ServerMessage::PrivilegeRevoked => {}
        other => panic!("Expected PrivilegeRevoked, got {:?}", other),
    }
}
