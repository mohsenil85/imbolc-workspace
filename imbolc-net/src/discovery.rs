//! mDNS/Bonjour discovery for Imbolc servers on LAN.
//!
//! Allows servers to advertise themselves and clients to discover available sessions.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

#[allow(unused_imports)]
use flume;
use log::{error, info, warn};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

/// Service type for Imbolc mDNS discovery.
pub const SERVICE_TYPE: &str = "_imbolc._tcp.local.";

/// Protocol version for compatibility checking.
const PROTOCOL_VERSION: &str = "1";

/// A discovered Imbolc server on the network.
#[derive(Debug, Clone)]
pub struct DiscoveredServer {
    /// Hostname of the server.
    pub host: String,
    /// Port number.
    pub port: u16,
    /// Session name.
    pub session_name: String,
    /// Number of connected clients.
    pub client_count: usize,
    /// Full address string for connection.
    pub address: String,
}

/// Server-side mDNS advertisement.
pub struct DiscoveryServer {
    daemon: ServiceDaemon,
    service_fullname: String,
    session_name: String,
    port: u16,
}

impl DiscoveryServer {
    /// Create and register a new discovery server.
    pub fn new(session_name: &str, port: u16) -> Result<Self, String> {
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;

        // Get hostname for instance name
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        let instance_name = format!("{}-{}", hostname, port);

        // Build TXT records
        let mut properties = HashMap::new();
        properties.insert("v".to_string(), PROTOCOL_VERSION.to_string());
        properties.insert("name".to_string(), session_name.to_string());
        properties.insert("clients".to_string(), "0".to_string());
        properties.insert("host".to_string(), hostname.clone());

        let service = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &format!("{}.local.", hostname),
            (),
            port,
            properties,
        )
        .map_err(|e| format!("Failed to create service info: {}", e))?;

        let fullname = service.get_fullname().to_string();

        daemon
            .register(service)
            .map_err(|e| format!("Failed to register service: {}", e))?;

        info!(
            "mDNS discovery registered: {} on port {} ({})",
            session_name, port, fullname
        );

        Ok(Self {
            daemon,
            service_fullname: fullname,
            session_name: session_name.to_string(),
            port,
        })
    }

    /// Update the client count in the TXT record.
    pub fn update_client_count(&self, count: usize) {
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        let instance_name = format!("{}-{}", hostname, self.port);

        let mut properties = HashMap::new();
        properties.insert("v".to_string(), PROTOCOL_VERSION.to_string());
        properties.insert("name".to_string(), self.session_name.clone());
        properties.insert("clients".to_string(), count.to_string());
        properties.insert("host".to_string(), hostname.clone());

        let service = match ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &format!("{}.local.", hostname),
            (),
            self.port,
            properties,
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to update service info: {}", e);
                return;
            }
        };

        if let Err(e) = self.daemon.register(service) {
            warn!("Failed to re-register service: {}", e);
        }
    }
}

impl Drop for DiscoveryServer {
    fn drop(&mut self) {
        if let Err(e) = self.daemon.unregister(&self.service_fullname) {
            warn!("Failed to unregister mDNS service: {}", e);
        }
        info!("mDNS discovery unregistered");
    }
}

/// Client-side mDNS browser for discovering servers.
pub struct DiscoveryClient {
    receiver: Receiver<DiscoveredServer>,
    _handle: thread::JoinHandle<()>,
    stop_tx: Sender<()>,
}

impl DiscoveryClient {
    /// Start browsing for Imbolc servers.
    pub fn new() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let (stop_tx, stop_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            if let Err(e) = browse_services(tx, stop_rx) {
                error!("mDNS browser error: {}", e);
            }
        });

        Ok(Self {
            receiver: rx,
            _handle: handle,
            stop_tx,
        })
    }

    /// Poll for discovered servers.
    pub fn poll(&self) -> Vec<DiscoveredServer> {
        let mut servers = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(server) => servers.push(server),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        servers
    }

    /// Browse for a specific duration and return all discovered servers.
    pub fn browse_for(duration: Duration) -> Result<Vec<DiscoveredServer>, String> {
        let client = Self::new()?;
        thread::sleep(duration);
        Ok(client.poll())
    }
}

impl Default for DiscoveryClient {
    fn default() -> Self {
        Self::new().expect("Failed to create discovery client")
    }
}

impl Drop for DiscoveryClient {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
    }
}

/// Background thread that browses for services.
fn browse_services(tx: Sender<DiscoveredServer>, stop_rx: Receiver<()>) -> Result<(), String> {
    let daemon =
        ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;

    let receiver = daemon
        .browse(SERVICE_TYPE)
        .map_err(|e| format!("Failed to browse: {}", e))?;

    info!("mDNS browser started for {}", SERVICE_TYPE);

    loop {
        // Check for stop signal
        if stop_rx.try_recv().is_ok() {
            break;
        }

        // Poll for events with timeout (flume uses its own RecvTimeoutError)
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if let ServiceEvent::ServiceResolved(info) = event {
                    let server = service_to_discovered(&info);
                    if let Some(server) = server {
                        info!(
                            "Discovered server: {} at {}",
                            server.session_name, server.address
                        );
                        if tx.send(server).is_err() {
                            break;
                        }
                    }
                }
            }
            Err(flume::RecvTimeoutError::Timeout) => {}
            Err(flume::RecvTimeoutError::Disconnected) => break,
        }
    }

    info!("mDNS browser stopped");
    Ok(())
}

/// Convert a resolved service to a DiscoveredServer.
fn service_to_discovered(info: &ServiceInfo) -> Option<DiscoveredServer> {
    let properties = info.get_properties();

    // Check protocol version
    let version = properties.get_property_val_str("v")?;
    if version != PROTOCOL_VERSION {
        warn!(
            "Ignoring server with incompatible protocol version: {} (expected {})",
            version, PROTOCOL_VERSION
        );
        return None;
    }

    let session_name = properties
        .get_property_val_str("name")
        .unwrap_or("untitled")
        .to_string();

    let client_count: usize = properties
        .get_property_val_str("clients")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let host = properties
        .get_property_val_str("host")
        .unwrap_or_else(|| info.get_hostname())
        .to_string();

    let port = info.get_port();

    // Get the first IP address
    let addresses = info.get_addresses();
    let ip = addresses.iter().next()?;
    let address = format!("{}:{}", ip, port);

    Some(DiscoveredServer {
        host,
        port,
        session_name,
        client_count,
        address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovered_server_display() {
        let server = DiscoveredServer {
            host: "test-machine".into(),
            port: 9999,
            session_name: "My Session".into(),
            client_count: 2,
            address: "192.168.1.100:9999".into(),
        };
        assert_eq!(server.session_name, "My Session");
        assert_eq!(server.client_count, 2);
    }
}
