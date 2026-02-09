//! Serialization roundtrip tests for all protocol message types.

use imbolc_net::protocol::*;
use imbolc_types::{InstrumentAction, ServerAction, SessionState, InstrumentState, SourceType};
use std::collections::HashMap;

fn roundtrip_client(msg: &ClientMessage) -> ClientMessage {
    let json = serde_json::to_string(msg).expect("serialize ClientMessage");
    serde_json::from_str(&json).expect("deserialize ClientMessage")
}

fn roundtrip_server(msg: &ServerMessage) -> ServerMessage {
    let json = serde_json::to_string(msg).expect("serialize ServerMessage");
    serde_json::from_str(&json).expect("deserialize ServerMessage")
}

fn make_network_state() -> NetworkState {
    let mut instruments = InstrumentState::new();
    instruments.add_instrument(SourceType::Saw);
    let mut ownership = HashMap::new();
    ownership.insert(0, OwnerInfo {
        client_id: ClientId::new(1),
        client_name: "Alice".into(),
    });
    NetworkState {
        session: SessionState::new(),
        instruments,
        ownership,
        privileged_client: Some((ClientId::new(1), "Alice".into())),
    }
}

// --- ClientMessage roundtrips ---

#[test]
fn test_roundtrip_client_hello() {
    let msg = ClientMessage::Hello {
        client_name: "Alice".into(),
        requested_instruments: vec![0, 1, 2],
        request_privilege: true,
        reconnect_token: None,
    };
    let rt = roundtrip_client(&msg);
    match rt {
        ClientMessage::Hello { client_name, requested_instruments, request_privilege, reconnect_token } => {
            assert_eq!(client_name, "Alice");
            assert_eq!(requested_instruments, vec![0, 1, 2]);
            assert!(request_privilege);
            assert!(reconnect_token.is_none());
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_client_hello_with_token() {
    let token = SessionToken("test-token-123".into());
    let msg = ClientMessage::Hello {
        client_name: "Bob".into(),
        requested_instruments: vec![],
        request_privilege: false,
        reconnect_token: Some(token.clone()),
    };
    let rt = roundtrip_client(&msg);
    match rt {
        ClientMessage::Hello { reconnect_token, .. } => {
            assert_eq!(reconnect_token, Some(token));
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_client_action() {
    let msg = ClientMessage::Action(NetworkAction::Instrument(InstrumentAction::Select(2)));
    let rt = roundtrip_client(&msg);
    match rt {
        ClientMessage::Action(NetworkAction::Instrument(InstrumentAction::Select(id))) => {
            assert_eq!(id, 2);
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_client_goodbye() {
    let msg = ClientMessage::Goodbye;
    let json = serde_json::to_string(&msg).unwrap();
    let rt: ClientMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(rt, ClientMessage::Goodbye));
}

#[test]
fn test_roundtrip_client_ping() {
    let msg = ClientMessage::Ping;
    let rt = roundtrip_client(&msg);
    assert!(matches!(rt, ClientMessage::Ping));
}

#[test]
fn test_roundtrip_client_pong() {
    let msg = ClientMessage::Pong;
    let rt = roundtrip_client(&msg);
    assert!(matches!(rt, ClientMessage::Pong));
}

#[test]
fn test_roundtrip_client_request_privilege() {
    let msg = ClientMessage::RequestPrivilege;
    let rt = roundtrip_client(&msg);
    assert!(matches!(rt, ClientMessage::RequestPrivilege));
}

#[test]
fn test_roundtrip_client_request_full_sync() {
    let msg = ClientMessage::RequestFullSync;
    let rt = roundtrip_client(&msg);
    assert!(matches!(rt, ClientMessage::RequestFullSync));
}

// --- ServerMessage roundtrips ---

#[test]
fn test_roundtrip_server_welcome() {
    let state = make_network_state();
    let msg = ServerMessage::Welcome {
        client_id: ClientId::new(42),
        granted_instruments: vec![0, 1],
        state,
        privilege: PrivilegeLevel::Privileged,
        session_token: SessionToken("tok-123".into()),
    };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::Welcome { client_id, granted_instruments, privilege, session_token, .. } => {
            assert_eq!(client_id, ClientId::new(42));
            assert_eq!(granted_instruments, vec![0, 1]);
            assert_eq!(privilege, PrivilegeLevel::Privileged);
            assert_eq!(session_token, SessionToken("tok-123".into()));
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_server_state_update() {
    let state = make_network_state();
    let msg = ServerMessage::StateUpdate { state };
    let rt = roundtrip_server(&msg);
    assert!(matches!(rt, ServerMessage::StateUpdate { .. }));
}

#[test]
fn test_roundtrip_server_metering() {
    let msg = ServerMessage::Metering {
        playhead: 1024,
        bpm: 120.5,
        peaks: (0.8, 0.7),
    };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::Metering { playhead, bpm, peaks } => {
            assert_eq!(playhead, 1024);
            assert!((bpm - 120.5).abs() < 0.01);
            assert!((peaks.0 - 0.8).abs() < 0.01);
            assert!((peaks.1 - 0.7).abs() < 0.01);
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_server_shutdown() {
    let msg = ServerMessage::Shutdown;
    let rt = roundtrip_server(&msg);
    assert!(matches!(rt, ServerMessage::Shutdown));
}

#[test]
fn test_roundtrip_server_ping() {
    let msg = ServerMessage::Ping;
    let rt = roundtrip_server(&msg);
    assert!(matches!(rt, ServerMessage::Ping));
}

#[test]
fn test_roundtrip_server_pong() {
    let msg = ServerMessage::Pong;
    let rt = roundtrip_server(&msg);
    assert!(matches!(rt, ServerMessage::Pong));
}

#[test]
fn test_roundtrip_server_error() {
    let msg = ServerMessage::Error { message: "Something went wrong".into() };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::Error { message } => assert_eq!(message, "Something went wrong"),
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_server_action_rejected() {
    let msg = ServerMessage::ActionRejected { reason: "No privilege".into() };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::ActionRejected { reason } => assert_eq!(reason, "No privilege"),
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_server_privilege_granted() {
    let msg = ServerMessage::PrivilegeGranted;
    let rt = roundtrip_server(&msg);
    assert!(matches!(rt, ServerMessage::PrivilegeGranted));
}

#[test]
fn test_roundtrip_server_privilege_denied() {
    let msg = ServerMessage::PrivilegeDenied { held_by: "Alice".into() };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::PrivilegeDenied { held_by } => assert_eq!(held_by, "Alice"),
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_server_privilege_revoked() {
    let msg = ServerMessage::PrivilegeRevoked;
    let rt = roundtrip_server(&msg);
    assert!(matches!(rt, ServerMessage::PrivilegeRevoked));
}

#[test]
fn test_roundtrip_server_reconnect_successful() {
    let msg = ServerMessage::ReconnectSuccessful {
        client_id: ClientId::new(7),
        restored_instruments: vec![0, 2, 4],
        privilege: PrivilegeLevel::Normal,
    };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::ReconnectSuccessful { client_id, restored_instruments, privilege } => {
            assert_eq!(client_id, ClientId::new(7));
            assert_eq!(restored_instruments, vec![0, 2, 4]);
            assert_eq!(privilege, PrivilegeLevel::Normal);
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_server_reconnect_failed() {
    let msg = ServerMessage::ReconnectFailed { reason: "Token expired".into() };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::ReconnectFailed { reason } => assert_eq!(reason, "Token expired"),
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_state_patch_update() {
    let patch = StatePatch {
        session: Some(SessionState::new()),
        instruments: None,
        ownership: None,
        privileged_client: Some(Some((ClientId::new(1), "Alice".into()))),
        seq: 42,
    };
    let msg = ServerMessage::StatePatchUpdate { patch };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::StatePatchUpdate { patch } => {
            assert!(patch.session.is_some());
            assert!(patch.instruments.is_none());
            assert!(patch.ownership.is_none());
            assert!(patch.privileged_client.is_some());
            assert_eq!(patch.seq, 42);
        }
        _ => panic!("Roundtrip failed"),
    }
}

#[test]
fn test_roundtrip_full_state_sync() {
    let state = make_network_state();
    let msg = ServerMessage::FullStateSync { state, seq: 100 };
    let rt = roundtrip_server(&msg);
    match rt {
        ServerMessage::FullStateSync { seq, .. } => {
            assert_eq!(seq, 100);
        }
        _ => panic!("Roundtrip failed"),
    }
}

// --- NetworkAction roundtrips ---

#[test]
fn test_roundtrip_network_action_variants() {
    let actions: Vec<NetworkAction> = vec![
        NetworkAction::None,
        NetworkAction::Quit,
        NetworkAction::Instrument(InstrumentAction::Select(0)),
        NetworkAction::Server(ServerAction::RecordMaster),
        NetworkAction::Undo,
        NetworkAction::Redo,
    ];
    for action in &actions {
        let json = serde_json::to_string(action).expect("serialize");
        let rt: NetworkAction = serde_json::from_str(&json).expect("deserialize");
        // Just verify it doesn't panic — we can't easily check equality without PartialEq
        let _ = rt;
    }
}

// --- StatePatch roundtrip ---

#[test]
fn test_roundtrip_state_patch_all_none() {
    let patch = StatePatch {
        session: None,
        instruments: None,
        ownership: None,
        privileged_client: None,
        seq: 0,
    };
    let json = serde_json::to_string(&patch).unwrap();
    let rt: StatePatch = serde_json::from_str(&json).unwrap();
    assert!(rt.session.is_none());
    assert!(rt.instruments.is_none());
    assert!(rt.ownership.is_none());
    assert!(rt.privileged_client.is_none());
    assert_eq!(rt.seq, 0);
}

/// `Some(None)` = "privilege revoked" must survive JSON roundtrip.
#[test]
fn test_roundtrip_state_patch_privileged_client_cleared() {
    let patch = StatePatch {
        session: None,
        instruments: None,
        ownership: None,
        privileged_client: Some(None), // "changed to: nobody"
        seq: 5,
    };
    let json = serde_json::to_string(&patch).unwrap();
    let rt: StatePatch = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.privileged_client, Some(None), "Some(None) must roundtrip");
    assert_eq!(rt.seq, 5);
}

/// All three privileged_client states must be distinguishable after roundtrip.
#[test]
fn test_roundtrip_state_patch_privileged_client_all_variants() {
    // None = no change
    let p1 = StatePatch { session: None, instruments: None, ownership: None, privileged_client: None, seq: 1 };
    let j1 = serde_json::to_string(&p1).unwrap();
    let r1: StatePatch = serde_json::from_str(&j1).unwrap();
    assert_eq!(r1.privileged_client, None);

    // Some(None) = changed to nobody
    let p2 = StatePatch { session: None, instruments: None, ownership: None, privileged_client: Some(None), seq: 2 };
    let j2 = serde_json::to_string(&p2).unwrap();
    let r2: StatePatch = serde_json::from_str(&j2).unwrap();
    assert_eq!(r2.privileged_client, Some(None));

    // Some(Some(...)) = changed to Alice
    let p3 = StatePatch {
        session: None, instruments: None, ownership: None,
        privileged_client: Some(Some((ClientId::new(1), "Alice".into()))),
        seq: 3,
    };
    let j3 = serde_json::to_string(&p3).unwrap();
    let r3: StatePatch = serde_json::from_str(&j3).unwrap();
    assert!(matches!(r3.privileged_client, Some(Some(_))));
}
