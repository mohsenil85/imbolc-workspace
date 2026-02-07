mod common;

use std::time::Duration;
use imbolc_net::server::NetServer;
use imbolc_net::protocol::{PrivilegeLevel, ServerMessage};

#[test]
fn test_connect_and_receive_welcome() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut client = common::RawClient::connect(&addr).unwrap();
    client.send_hello("Alice", vec![], false).unwrap();

    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let welcome = client.recv().unwrap();
    match welcome {
        ServerMessage::Welcome { granted_instruments, privilege, .. } => {
            assert_eq!(granted_instruments.len(), 0);
            assert_eq!(privilege, PrivilegeLevel::Normal);
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }
}

#[test]
fn test_ownership_granted_on_connect() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state_with_instruments(&server, 3);

    let mut client = common::RawClient::connect(&addr).unwrap();
    client.send_hello("Alice", vec![0, 1], false).unwrap();

    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let welcome = client.recv().unwrap();
    match welcome {
        ServerMessage::Welcome { granted_instruments, .. } => {
            assert_eq!(granted_instruments.len(), 2);
            assert!(granted_instruments.contains(&0));
            assert!(granted_instruments.contains(&1));
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }
}

#[test]
fn test_privilege_granted_on_connect() {
    let mut server = NetServer::bind("127.0.0.1:0").unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let state = common::make_test_state(&server);

    let mut client = common::RawClient::connect(&addr).unwrap();
    client.send_hello("Alice", vec![], true).unwrap();

    common::drive_until_clients(&mut server, &state, 1, Duration::from_secs(2));

    let welcome = client.recv().unwrap();
    match welcome {
        ServerMessage::Welcome { privilege, .. } => {
            assert_eq!(privilege, PrivilegeLevel::Privileged);
        }
        other => panic!("Expected Welcome, got {:?}", other),
    }
}
