//! Dedicated OSC send thread for timed bundles.
//!
//! Timed bundles (voice spawns, automation, clicks) are pre-encoded on the audio
//! thread and pushed to a bounded channel. A dedicated sender thread drains the
//! channel and performs `socket.send_to()`, keeping UDP I/O off the audio thread.

use std::net::{SocketAddr, UdpSocket};
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, TrySendError};

/// A pre-encoded OSC bundle ready for UDP transmission.
pub struct OscSendEntry {
    pub encoded_bundle: Vec<u8>,
}

/// Channel capacity for the OSC send queue.
/// At 2000 ticks/sec × ~10 events/tick = 20K events/sec; sender drains faster.
const SEND_QUEUE_CAPACITY: usize = 512;

/// Create the sender channel pair and spawn the sender thread.
///
/// Returns `(Sender, queue_len_fn, JoinHandle)`.
/// The `queue_len_fn` returns current queue depth for telemetry.
pub fn spawn_osc_sender(
    socket: UdpSocket,
    server_addr: SocketAddr,
) -> (
    Sender<OscSendEntry>,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
    JoinHandle<()>,
) {
    let (tx, rx) = crossbeam_channel::bounded::<OscSendEntry>(SEND_QUEUE_CAPACITY);
    let queue_depth = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let depth_clone = queue_depth.clone();

    let handle = thread::Builder::new()
        .name("osc-sender".into())
        .spawn(move || {
            sender_loop(socket, server_addr, rx, depth_clone);
        })
        .expect("failed to spawn osc-sender thread");

    (tx, queue_depth, handle)
}

fn sender_loop(
    socket: UdpSocket,
    server_addr: SocketAddr,
    rx: Receiver<OscSendEntry>,
    queue_depth: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) {
    while let Ok(entry) = rx.recv() {
        queue_depth.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        let _ = socket.send_to(&entry.encoded_bundle, server_addr);
    }
}

/// Try to push a pre-encoded bundle to the sender thread.
/// Returns `true` if queued, `false` if the channel was full (caller should fall back).
pub fn try_queue_bundle(
    tx: &Sender<OscSendEntry>,
    queue_depth: &std::sync::atomic::AtomicUsize,
    encoded_bundle: Vec<u8>,
) -> bool {
    match tx.try_send(OscSendEntry { encoded_bundle }) {
        Ok(()) => {
            queue_depth.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            true
        }
        Err(TrySendError::Full(_)) => {
            log::warn!(target: "audio::osc_sender", "OSC send queue full, falling back to direct send");
            false
        }
        Err(TrySendError::Disconnected(_)) => false,
    }
}
