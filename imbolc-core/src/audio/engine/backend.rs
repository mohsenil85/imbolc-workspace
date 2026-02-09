//! Audio backend trait: a semantic-level abstraction over audio server operations.
//!
//! `AudioBackend` captures what the engine *means* to do (create a synth, free a node,
//! set a parameter) independently of how it's done (OSC messages to SuperCollider).
//! This enables unit testing of routing logic without a running audio server.

use std::fmt;
use std::path::Path;

/// Result type for backend operations.
pub type BackendResult<T = ()> = Result<T, BackendError>;

/// Error from a backend operation.
#[derive(Debug, Clone)]
pub struct BackendError(pub String);

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BackendError {}

impl From<std::io::Error> for BackendError {
    fn from(e: std::io::Error) -> Self {
        BackendError(e.to_string())
    }
}

impl From<String> for BackendError {
    fn from(s: String) -> Self {
        BackendError(s)
    }
}

/// Protocol-agnostic message for bundled operations.
/// Replaces `rosc::OscMessage` in engine code.
#[derive(Debug, Clone, PartialEq)]
pub struct BackendMessage {
    pub addr: String,
    pub args: Vec<RawArg>,
}

/// Sentinel value for `offset_secs` meaning "execute immediately".
/// `ScBackend` maps this to NTP timetag `(0, 1)`.
pub const BUNDLE_IMMEDIATE: f64 = -1.0;

/// Build an /n_set message for a single parameter on a node.
pub fn build_n_set_message(node_id: i32, param: &str, value: f32) -> BackendMessage {
    BackendMessage {
        addr: "/n_set".to_string(),
        args: vec![
            RawArg::Int(node_id),
            RawArg::Str(param.to_string()),
            RawArg::Float(value),
        ],
    }
}

/// Semantic-level audio backend trait.
///
/// Each method represents a meaningful audio operation. Implementations
/// translate these into server-specific commands (e.g., OSC for SuperCollider)
/// or record them for testing.
pub trait AudioBackend: Send {
    /// Create a group node for execution ordering.
    fn create_group(&self, group_id: i32, add_action: i32, target: i32) -> BackendResult;

    /// Create a synth in a specific group with named parameters.
    fn create_synth(
        &self,
        def_name: &str,
        node_id: i32,
        group_id: i32,
        params: &[(String, f32)],
    ) -> BackendResult;

    /// Free (remove) a node from the server.
    fn free_node(&self, node_id: i32) -> BackendResult;

    /// Set a single parameter on a node.
    fn set_param(&self, node_id: i32, param: &str, value: f32) -> BackendResult;

    /// Set multiple parameters on a node atomically.
    fn set_params(&self, node_id: i32, params: &[(&str, f32)]) -> BackendResult;

    /// Set multiple parameters on a node as a timestamped bundle.
    fn set_params_bundled(&self, node_id: i32, params: &[(&str, f32)], offset_secs: f64) -> BackendResult;

    /// Send multiple messages as a single timestamped bundle.
    fn send_bundle(&self, messages: Vec<BackendMessage>, offset_secs: f64) -> BackendResult;

    /// Send a unit command to a specific UGen instance within a synth node.
    fn send_unit_cmd(&self, node_id: i32, ugen_index: i32, cmd: &str, args: Vec<RawArg>) -> BackendResult;

    /// Load a sound file into a buffer at the given buffer number.
    fn load_buffer(&self, bufnum: i32, path: &Path) -> BackendResult;

    /// Free a buffer.
    fn free_buffer(&self, bufnum: i32) -> BackendResult;

    /// Allocate an empty buffer with the given frame count and channel count.
    fn alloc_buffer(&self, bufnum: i32, num_frames: i32, num_channels: i32) -> BackendResult;

    /// Open a buffer for disk writing.
    fn open_buffer_for_write(&self, bufnum: i32, path: &Path) -> BackendResult;

    /// Close a buffer's soundfile.
    fn close_buffer(&self, bufnum: i32) -> BackendResult;

    /// Query buffer info.
    fn query_buffer(&self, bufnum: i32) -> BackendResult;

    /// Send a raw message (escape hatch for operations not covered by typed methods).
    fn send_raw(&self, addr: &str, args: Vec<RawArg>) -> BackendResult;

    /// Clone the underlying UDP socket for the OSC sender thread.
    /// Returns None for test/null backends.
    fn try_clone_socket(&self) -> Option<std::net::UdpSocket> { None }

    /// Get the server's socket address for the OSC sender thread.
    /// Returns None for test/null backends.
    fn server_socket_addr(&self) -> Option<std::net::SocketAddr> { None }
}

/// A loosely-typed argument for backend messages, so engine code doesn't depend on `rosc`.
#[derive(Debug, Clone, PartialEq)]
pub enum RawArg {
    Int(i32),
    Float(f32),
    Str(String),
    Blob(Vec<u8>),
}

// ─── SuperCollider Backend ──────────────────────────────────────────

use super::super::osc_client::{osc_time_from_now, osc_time_immediate, OscClient};

/// Backend implementation that delegates to `OscClient` for SuperCollider communication.
pub struct ScBackend {
    client: OscClient,
}

impl ScBackend {
    pub fn new(client: OscClient) -> Self {
        Self { client }
    }
}

/// Convert `RawArg` to `rosc::OscType` (public, for `queue_timed_bundle` encoding).
pub fn raw_to_osc_pub(arg: RawArg) -> rosc::OscType {
    raw_to_osc(arg)
}

/// Convert `RawArg` to `rosc::OscType`.
fn raw_to_osc(arg: RawArg) -> rosc::OscType {
    match arg {
        RawArg::Int(v) => rosc::OscType::Int(v),
        RawArg::Float(v) => rosc::OscType::Float(v),
        RawArg::Str(v) => rosc::OscType::String(v),
        RawArg::Blob(v) => rosc::OscType::Blob(v),
    }
}

/// Convert `offset_secs` to an OSC timetag.
/// Negative values (BUNDLE_IMMEDIATE) map to the "immediate" timetag.
fn offset_to_osc_time(offset_secs: f64) -> rosc::OscTime {
    if offset_secs < 0.0 {
        osc_time_immediate()
    } else {
        osc_time_from_now(offset_secs)
    }
}

impl AudioBackend for ScBackend {
    fn create_group(&self, group_id: i32, add_action: i32, target: i32) -> BackendResult {
        self.client
            .create_group(group_id, add_action, target)
            .map_err(BackendError::from)
    }

    fn create_synth(
        &self,
        def_name: &str,
        node_id: i32,
        group_id: i32,
        params: &[(String, f32)],
    ) -> BackendResult {
        self.client
            .create_synth_in_group(def_name, node_id, group_id, params)
            .map_err(BackendError::from)
    }

    fn free_node(&self, node_id: i32) -> BackendResult {
        self.client.free_node(node_id).map_err(BackendError::from)
    }

    fn set_param(&self, node_id: i32, param: &str, value: f32) -> BackendResult {
        self.client
            .set_param(node_id, param, value)
            .map_err(BackendError::from)
    }

    fn set_params(&self, node_id: i32, params: &[(&str, f32)]) -> BackendResult {
        let mut args = vec![rosc::OscType::Int(node_id)];
        for &(name, value) in params {
            args.push(rosc::OscType::String(name.to_string()));
            args.push(rosc::OscType::Float(value));
        }
        self.client
            .send_message("/n_set", args)
            .map_err(BackendError::from)
    }

    fn set_params_bundled(&self, node_id: i32, params: &[(&str, f32)], offset_secs: f64) -> BackendResult {
        let time = offset_to_osc_time(offset_secs);
        self.client
            .set_params_bundled(node_id, params, time)
            .map_err(BackendError::from)
    }

    fn send_bundle(&self, messages: Vec<BackendMessage>, offset_secs: f64) -> BackendResult {
        let time = offset_to_osc_time(offset_secs);
        let osc_messages: Vec<rosc::OscMessage> = messages
            .into_iter()
            .map(|m| rosc::OscMessage {
                addr: m.addr,
                args: m.args.into_iter().map(raw_to_osc).collect(),
            })
            .collect();
        self.client
            .send_bundle(osc_messages, time)
            .map_err(BackendError::from)
    }

    fn send_unit_cmd(&self, node_id: i32, ugen_index: i32, cmd: &str, args: Vec<RawArg>) -> BackendResult {
        let osc_args: Vec<rosc::OscType> = args.into_iter().map(raw_to_osc).collect();
        self.client
            .send_unit_cmd(node_id, ugen_index, cmd, osc_args)
            .map_err(BackendError::from)
    }

    fn load_buffer(&self, bufnum: i32, path: &Path) -> BackendResult {
        let path_str = path.to_string_lossy();
        self.client
            .load_buffer(bufnum, &path_str)
            .map_err(BackendError::from)
    }

    fn free_buffer(&self, bufnum: i32) -> BackendResult {
        self.client
            .free_buffer(bufnum)
            .map_err(BackendError::from)
    }

    fn alloc_buffer(&self, bufnum: i32, num_frames: i32, num_channels: i32) -> BackendResult {
        self.client
            .alloc_buffer(bufnum, num_frames, num_channels)
            .map_err(BackendError::from)
    }

    fn open_buffer_for_write(&self, bufnum: i32, path: &Path) -> BackendResult {
        self.client
            .open_buffer_for_write(bufnum, &path.to_string_lossy())
            .map_err(BackendError::from)
    }

    fn close_buffer(&self, bufnum: i32) -> BackendResult {
        self.client
            .close_buffer(bufnum)
            .map_err(BackendError::from)
    }

    fn query_buffer(&self, bufnum: i32) -> BackendResult {
        self.client
            .query_buffer(bufnum)
            .map_err(BackendError::from)
    }

    fn send_raw(&self, addr: &str, args: Vec<RawArg>) -> BackendResult {
        let osc_args: Vec<rosc::OscType> = args.into_iter().map(raw_to_osc).collect();
        self.client
            .send_message(addr, osc_args)
            .map_err(BackendError::from)
    }

    fn try_clone_socket(&self) -> Option<std::net::UdpSocket> {
        self.client.try_clone_socket().ok()
    }

    fn server_socket_addr(&self) -> Option<std::net::SocketAddr> {
        self.client.server_socket_addr().ok()
    }
}

// ─── Test Backend ───────────────────────────────────────────────────

use std::sync::{Arc, Mutex};

/// An operation recorded by `TestBackend` for assertion in tests.
#[derive(Debug, Clone, PartialEq)]
pub enum TestOp {
    CreateGroup {
        group_id: i32,
        add_action: i32,
        target: i32,
    },
    CreateSynth {
        def_name: String,
        node_id: i32,
        group_id: i32,
        params: Vec<(String, f32)>,
    },
    FreeNode(i32),
    SetParam {
        node_id: i32,
        param: String,
        value: f32,
    },
    SetParams {
        node_id: i32,
        params: Vec<(String, f32)>,
    },
    SetParamsBundled {
        node_id: i32,
        params: Vec<(String, f32)>,
        offset_secs: f64,
    },
    SendBundle {
        messages: Vec<(String, Vec<RawArg>)>,
        offset_secs: f64,
    },
    SendUnitCmd {
        node_id: i32,
        ugen_index: i32,
        cmd: String,
        args: Vec<RawArg>,
    },
    LoadBuffer {
        bufnum: i32,
        path: String,
    },
    FreeBuffer(i32),
    AllocBuffer {
        bufnum: i32,
        num_frames: i32,
        num_channels: i32,
    },
    OpenBufferForWrite {
        bufnum: i32,
        path: String,
    },
    CloseBuffer(i32),
    QueryBuffer(i32),
    SendRaw {
        addr: String,
        args: Vec<RawArg>,
    },
}

/// A test backend that records all operations into a vector for assertions.
/// All operations succeed by default. Uses `Mutex` for interior mutability
/// so the backend is `Send + Sync` (needed for `Arc<TestBackend>` sharing).
pub struct TestBackend {
    ops: Mutex<Vec<TestOp>>,
}

impl TestBackend {
    pub fn new() -> Self {
        Self {
            ops: Mutex::new(Vec::new()),
        }
    }

    /// Return all recorded operations.
    pub fn operations(&self) -> Vec<TestOp> {
        self.ops.lock().unwrap().clone()
    }

    /// Clear recorded operations.
    pub fn clear(&self) {
        self.ops.lock().unwrap().clear();
    }

    /// Count operations matching a predicate.
    pub fn count<F: Fn(&TestOp) -> bool>(&self, f: F) -> usize {
        self.ops.lock().unwrap().iter().filter(|op| f(op)).count()
    }

    /// Find the first operation matching a predicate.
    pub fn find<F: Fn(&TestOp) -> bool>(&self, f: F) -> Option<TestOp> {
        self.ops.lock().unwrap().iter().find(|op| f(op)).cloned()
    }

    /// Return all CreateSynth operations.
    pub fn synths_created(&self) -> Vec<TestOp> {
        self.ops
            .lock()
            .unwrap()
            .iter()
            .filter(|op| matches!(op, TestOp::CreateSynth { .. }))
            .cloned()
            .collect()
    }

    /// Return all FreeNode operations.
    pub fn nodes_freed(&self) -> Vec<i32> {
        self.ops
            .lock()
            .unwrap()
            .iter()
            .filter_map(|op| match op {
                TestOp::FreeNode(id) => Some(*id),
                _ => None,
            })
            .collect()
    }
}

impl AudioBackend for TestBackend {
    fn create_group(&self, group_id: i32, add_action: i32, target: i32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::CreateGroup {
            group_id,
            add_action,
            target,
        });
        Ok(())
    }

    fn create_synth(
        &self,
        def_name: &str,
        node_id: i32,
        group_id: i32,
        params: &[(String, f32)],
    ) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::CreateSynth {
            def_name: def_name.to_string(),
            node_id,
            group_id,
            params: params.to_vec(),
        });
        Ok(())
    }

    fn free_node(&self, node_id: i32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::FreeNode(node_id));
        Ok(())
    }

    fn set_param(&self, node_id: i32, param: &str, value: f32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::SetParam {
            node_id,
            param: param.to_string(),
            value,
        });
        Ok(())
    }

    fn set_params(&self, node_id: i32, params: &[(&str, f32)]) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::SetParams {
            node_id,
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        });
        Ok(())
    }

    fn set_params_bundled(&self, node_id: i32, params: &[(&str, f32)], offset_secs: f64) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::SetParamsBundled {
            node_id,
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            offset_secs,
        });
        Ok(())
    }

    fn send_bundle(&self, messages: Vec<BackendMessage>, offset_secs: f64) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::SendBundle {
            messages: messages.into_iter().map(|m| (m.addr, m.args)).collect(),
            offset_secs,
        });
        Ok(())
    }

    fn send_unit_cmd(&self, node_id: i32, ugen_index: i32, cmd: &str, args: Vec<RawArg>) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::SendUnitCmd {
            node_id,
            ugen_index,
            cmd: cmd.to_string(),
            args,
        });
        Ok(())
    }

    fn load_buffer(&self, bufnum: i32, path: &Path) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::LoadBuffer {
            bufnum,
            path: path.to_string_lossy().to_string(),
        });
        Ok(())
    }

    fn free_buffer(&self, bufnum: i32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::FreeBuffer(bufnum));
        Ok(())
    }

    fn alloc_buffer(&self, bufnum: i32, num_frames: i32, num_channels: i32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::AllocBuffer {
            bufnum,
            num_frames,
            num_channels,
        });
        Ok(())
    }

    fn open_buffer_for_write(&self, bufnum: i32, path: &Path) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::OpenBufferForWrite {
            bufnum,
            path: path.to_string_lossy().to_string(),
        });
        Ok(())
    }

    fn close_buffer(&self, bufnum: i32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::CloseBuffer(bufnum));
        Ok(())
    }

    fn query_buffer(&self, bufnum: i32) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::QueryBuffer(bufnum));
        Ok(())
    }

    fn send_raw(&self, addr: &str, args: Vec<RawArg>) -> BackendResult {
        self.ops.lock().unwrap().push(TestOp::SendRaw {
            addr: addr.to_string(),
            args,
        });
        Ok(())
    }
}

impl Default for TestBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Wraps `Arc<TestBackend>` to implement `AudioBackend` so the engine can
/// own a `Box<dyn AudioBackend>` while tests retain an `Arc` for assertions.
pub struct SharedTestBackend(pub Arc<TestBackend>);

impl AudioBackend for SharedTestBackend {
    fn create_group(&self, group_id: i32, add_action: i32, target: i32) -> BackendResult {
        self.0.create_group(group_id, add_action, target)
    }
    fn create_synth(&self, def_name: &str, node_id: i32, group_id: i32, params: &[(String, f32)]) -> BackendResult {
        self.0.create_synth(def_name, node_id, group_id, params)
    }
    fn free_node(&self, node_id: i32) -> BackendResult {
        self.0.free_node(node_id)
    }
    fn set_param(&self, node_id: i32, param: &str, value: f32) -> BackendResult {
        self.0.set_param(node_id, param, value)
    }
    fn set_params(&self, node_id: i32, params: &[(&str, f32)]) -> BackendResult {
        self.0.set_params(node_id, params)
    }
    fn set_params_bundled(&self, node_id: i32, params: &[(&str, f32)], offset_secs: f64) -> BackendResult {
        self.0.set_params_bundled(node_id, params, offset_secs)
    }
    fn send_bundle(&self, messages: Vec<BackendMessage>, offset_secs: f64) -> BackendResult {
        self.0.send_bundle(messages, offset_secs)
    }
    fn send_unit_cmd(&self, node_id: i32, ugen_index: i32, cmd: &str, args: Vec<RawArg>) -> BackendResult {
        self.0.send_unit_cmd(node_id, ugen_index, cmd, args)
    }
    fn load_buffer(&self, bufnum: i32, path: &Path) -> BackendResult {
        self.0.load_buffer(bufnum, path)
    }
    fn free_buffer(&self, bufnum: i32) -> BackendResult {
        self.0.free_buffer(bufnum)
    }
    fn alloc_buffer(&self, bufnum: i32, num_frames: i32, num_channels: i32) -> BackendResult {
        self.0.alloc_buffer(bufnum, num_frames, num_channels)
    }
    fn open_buffer_for_write(&self, bufnum: i32, path: &Path) -> BackendResult {
        self.0.open_buffer_for_write(bufnum, path)
    }
    fn close_buffer(&self, bufnum: i32) -> BackendResult {
        self.0.close_buffer(bufnum)
    }
    fn query_buffer(&self, bufnum: i32) -> BackendResult {
        self.0.query_buffer(bufnum)
    }
    fn send_raw(&self, addr: &str, args: Vec<RawArg>) -> BackendResult {
        self.0.send_raw(addr, args)
    }
}

// ─── NullBackend ────────────────────────────────────────────────────

/// A no-op backend that silently succeeds. Useful as a default when
/// no audio server is connected.
pub struct NullBackend;

impl AudioBackend for NullBackend {
    fn create_group(&self, _: i32, _: i32, _: i32) -> BackendResult { Ok(()) }
    fn create_synth(&self, _: &str, _: i32, _: i32, _: &[(String, f32)]) -> BackendResult { Ok(()) }
    fn free_node(&self, _: i32) -> BackendResult { Ok(()) }
    fn set_param(&self, _: i32, _: &str, _: f32) -> BackendResult { Ok(()) }
    fn set_params(&self, _: i32, _: &[(&str, f32)]) -> BackendResult { Ok(()) }
    fn set_params_bundled(&self, _: i32, _: &[(&str, f32)], _: f64) -> BackendResult { Ok(()) }
    fn send_bundle(&self, _: Vec<BackendMessage>, _: f64) -> BackendResult { Ok(()) }
    fn send_unit_cmd(&self, _: i32, _: i32, _: &str, _: Vec<RawArg>) -> BackendResult { Ok(()) }
    fn load_buffer(&self, _: i32, _: &Path) -> BackendResult { Ok(()) }
    fn free_buffer(&self, _: i32) -> BackendResult { Ok(()) }
    fn alloc_buffer(&self, _: i32, _: i32, _: i32) -> BackendResult { Ok(()) }
    fn open_buffer_for_write(&self, _: i32, _: &Path) -> BackendResult { Ok(()) }
    fn close_buffer(&self, _: i32) -> BackendResult { Ok(()) }
    fn query_buffer(&self, _: i32) -> BackendResult { Ok(()) }
    fn send_raw(&self, _: &str, _: Vec<RawArg>) -> BackendResult { Ok(()) }
}
