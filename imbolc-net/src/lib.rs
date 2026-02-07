//! Network layer for Imbolc multi-client collaboration.
//!
//! This crate provides client and server components for running Imbolc
//! sessions over LAN with multiple collaborators.

pub mod client;
pub mod framing;
pub mod protocol;
pub mod server;
pub mod session_file;

#[cfg(feature = "mdns")]
pub mod discovery;

pub use client::{MeteringUpdate, OwnershipStatus, RemoteDispatcher};
pub use protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, OwnerInfo, PrivilegeLevel,
    ServerMessage, SessionToken, StatePatch,
};
pub use server::{DirtyFlags, NetServer};
pub use session_file::{clear_session, load_session, save_session, SavedSession};

#[cfg(feature = "mdns")]
pub use discovery::{DiscoveredServer, DiscoveryClient, DiscoveryServer};
