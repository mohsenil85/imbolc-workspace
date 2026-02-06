//! Network layer for Imbolc multi-client collaboration.
//!
//! This crate provides client and server components for running Imbolc
//! sessions over LAN with multiple collaborators.

pub mod client;
pub mod framing;
pub mod protocol;
pub mod server;

#[cfg(feature = "mdns")]
pub mod discovery;

pub use client::{MeteringUpdate, OwnershipStatus, RemoteDispatcher};
pub use protocol::{
    ClientId, ClientMessage, NetworkAction, NetworkState, OwnerInfo, PrivilegeLevel,
    ServerMessage, SessionToken,
};
pub use server::NetServer;

#[cfg(feature = "mdns")]
pub use discovery::{DiscoveredServer, DiscoveryClient, DiscoveryServer};
