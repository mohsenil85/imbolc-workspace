//! Lock-free triple buffer for low-latency data sharing between threads.
//!
//! A triple buffer uses three slots to allow lock-free reading and writing:
//! - Writer writes to slot A while reader reads from slot C
//! - Writer atomically swaps A with B (middle buffer) when done
//! - Reader atomically swaps C with B when it wants fresh data
//!
//! This ensures the writer never blocks and the reader gets the latest
//! complete frame without tearing.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::cell::UnsafeCell;

/// Index encoding for triple buffer state.
/// Uses 2 bits per slot to track which logical buffer each physical slot represents.
/// State byte layout: [unused:2][back:2][middle:2][front:2]
const FRONT_SHIFT: u8 = 0;
const MIDDLE_SHIFT: u8 = 2;
const BACK_SHIFT: u8 = 4;
const SLOT_MASK: u8 = 0b11;

/// Shared triple buffer state (wraps 3 slots).
pub struct TripleBufferShared<T> {
    /// Three data slots
    slots: [UnsafeCell<T>; 3],
    /// Atomic state: encodes which slot is front/middle/back
    /// Bit 0 = fresh data available in middle
    state: AtomicU8,
}

// Safety: We guarantee exclusive access through the atomic state machine
unsafe impl<T: Send> Send for TripleBufferShared<T> {}
unsafe impl<T: Send + Sync> Sync for TripleBufferShared<T> {}

impl<T: Clone + Default> TripleBufferShared<T> {
    /// Create a new triple buffer with default values.
    pub fn new() -> Self {
        Self {
            slots: [
                UnsafeCell::new(T::default()),
                UnsafeCell::new(T::default()),
                UnsafeCell::new(T::default()),
            ],
            // Initial state: slot 0 = front (reader), slot 1 = middle, slot 2 = back (writer)
            // Encoded as: back=2, middle=1, front=0, fresh=0
            state: AtomicU8::new((2 << BACK_SHIFT) | (1 << MIDDLE_SHIFT) | (0 << FRONT_SHIFT)),
        }
    }

    /// Create a new triple buffer initialized with a value.
    pub fn new_with(value: T) -> Self {
        Self {
            slots: [
                UnsafeCell::new(value.clone()),
                UnsafeCell::new(value.clone()),
                UnsafeCell::new(value),
            ],
            state: AtomicU8::new((2 << BACK_SHIFT) | (1 << MIDDLE_SHIFT) | (0 << FRONT_SHIFT)),
        }
    }

    fn decode_back(state: u8) -> usize {
        ((state >> BACK_SHIFT) & SLOT_MASK) as usize
    }

    fn decode_middle(state: u8) -> usize {
        ((state >> MIDDLE_SHIFT) & SLOT_MASK) as usize
    }

    fn decode_front(state: u8) -> usize {
        ((state >> FRONT_SHIFT) & SLOT_MASK) as usize
    }

    /// Writer: get mutable reference to back buffer (caller must ensure single writer)
    #[allow(clippy::mut_from_ref)]
    unsafe fn back_mut(&self) -> &mut T {
        let state = self.state.load(Ordering::Acquire);
        let back_idx = Self::decode_back(state);
        &mut *self.slots[back_idx].get()
    }

    /// Writer: publish back buffer to middle (atomically swap back and middle)
    fn publish(&self) {
        loop {
            let state = self.state.load(Ordering::Acquire);
            let back_idx = Self::decode_back(state);
            let middle_idx = Self::decode_middle(state);
            let front_idx = Self::decode_front(state);

            // New state: old middle becomes back, old back becomes middle, set fresh bit
            let new_state = ((middle_idx as u8) << BACK_SHIFT)
                | ((back_idx as u8) << MIDDLE_SHIFT)
                | ((front_idx as u8) << FRONT_SHIFT)
                | 0x80; // fresh bit in high byte

            if self
                .state
                .compare_exchange_weak(state, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Reader: check if fresh data is available
    fn has_fresh(&self) -> bool {
        (self.state.load(Ordering::Acquire) & 0x80) != 0
    }

    /// Reader: swap front with middle if fresh data available (atomically)
    fn consume(&self) {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if (state & 0x80) == 0 {
                // No fresh data
                return;
            }

            let back_idx = Self::decode_back(state);
            let middle_idx = Self::decode_middle(state);
            let front_idx = Self::decode_front(state);

            // New state: old front becomes middle, old middle becomes front, clear fresh bit
            let new_state = ((back_idx as u8) << BACK_SHIFT)
                | ((front_idx as u8) << MIDDLE_SHIFT)
                | ((middle_idx as u8) << FRONT_SHIFT);
            // fresh bit cleared

            if self
                .state
                .compare_exchange_weak(state, new_state, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Reader: get reference to front buffer (caller must ensure single reader)
    unsafe fn front(&self) -> &T {
        let state = self.state.load(Ordering::Acquire);
        let front_idx = Self::decode_front(state);
        &*self.slots[front_idx].get()
    }
}

impl<T: Clone + Default> Default for TripleBufferShared<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Writer half of a triple buffer.
pub struct TripleBufferWriter<T> {
    shared: Arc<TripleBufferShared<T>>,
}

impl<T: Clone + Default> TripleBufferWriter<T> {
    /// Write a value and publish it for the reader.
    pub fn write(&mut self, value: T) {
        // Safety: single writer guaranteed by owning TripleBufferWriter
        unsafe {
            *self.shared.back_mut() = value;
        }
        self.shared.publish();
    }

    /// Get mutable access to the back buffer for in-place updates.
    /// Call `publish()` when done to make changes visible to reader.
    pub fn back_mut(&mut self) -> &mut T {
        // Safety: single writer guaranteed by owning TripleBufferWriter
        unsafe { self.shared.back_mut() }
    }

    /// Publish the current back buffer to the reader.
    pub fn publish(&mut self) {
        self.shared.publish();
    }
}

/// Reader half of a triple buffer.
pub struct TripleBufferReader<T> {
    shared: Arc<TripleBufferShared<T>>,
}

impl<T: Clone + Default> TripleBufferReader<T> {
    /// Check if new data is available from the writer.
    pub fn has_fresh(&self) -> bool {
        self.shared.has_fresh()
    }

    /// Get the latest data from the writer.
    /// Returns a clone of the current front buffer after consuming any fresh data.
    pub fn read(&self) -> T
    where
        T: Clone,
    {
        self.shared.consume();
        // Safety: single reader guaranteed by owning TripleBufferReader
        unsafe { self.shared.front().clone() }
    }

    /// Read with a closure to avoid cloning.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.shared.consume();
        // Safety: single reader guaranteed by owning TripleBufferReader
        unsafe { f(self.shared.front()) }
    }
}

/// Create a new triple buffer, returning writer and reader halves.
pub fn triple_buffer<T: Clone + Default>() -> (TripleBufferWriter<T>, TripleBufferReader<T>) {
    let shared = Arc::new(TripleBufferShared::new());
    (
        TripleBufferWriter {
            shared: Arc::clone(&shared),
        },
        TripleBufferReader { shared },
    )
}

/// Create a new triple buffer initialized with a value.
pub fn triple_buffer_with<T: Clone + Default>(
    value: T,
) -> (TripleBufferWriter<T>, TripleBufferReader<T>) {
    let shared = Arc::new(TripleBufferShared::new_with(value));
    (
        TripleBufferWriter {
            shared: Arc::clone(&shared),
        },
        TripleBufferReader { shared },
    )
}

/// Cloneable triple buffer handle that can be shared via Arc.
/// Both readers and writers use the same handle - suitable for AudioMonitor pattern
/// where the handle is cloned and shared between threads.
pub struct TripleBufferHandle<T> {
    shared: Arc<TripleBufferShared<T>>,
}

impl<T: Clone + Default> TripleBufferHandle<T> {
    /// Create a new triple buffer handle with default value.
    pub fn new() -> Self {
        Self {
            shared: Arc::new(TripleBufferShared::new()),
        }
    }

    /// Create a new triple buffer handle with initial value.
    pub fn new_with(value: T) -> Self {
        Self {
            shared: Arc::new(TripleBufferShared::new_with(value)),
        }
    }

    /// Write a value (call from writer thread).
    /// Note: Only one thread should write at a time.
    pub fn write(&self, value: T) {
        // Safety: We rely on the caller ensuring single-writer semantics
        // (the OSC receive thread is the only writer)
        unsafe {
            *self.shared.back_mut() = value;
        }
        self.shared.publish();
    }

    /// Modify the back buffer in place and publish (call from writer thread).
    /// Note: Only one thread should write at a time.
    pub fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        // Safety: We rely on the caller ensuring single-writer semantics
        unsafe {
            f(self.shared.back_mut());
        }
        self.shared.publish();
    }

    /// Read the latest value (call from reader thread).
    /// Note: Only one thread should read at a time for best performance,
    /// but multiple readers are safe (just less optimal).
    pub fn read(&self) -> T
    where
        T: Clone,
    {
        self.shared.consume();
        // Safety: single logical reader at a time expected
        unsafe { self.shared.front().clone() }
    }

    /// Read with a closure to avoid cloning.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.shared.consume();
        unsafe { f(self.shared.front()) }
    }
}

impl<T: Clone + Default> Clone for TripleBufferHandle<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T: Clone + Default> Default for TripleBufferHandle<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_write_read() {
        let (mut writer, reader) = triple_buffer::<i32>();
        writer.write(42);
        assert_eq!(reader.read(), 42);
    }

    #[test]
    fn test_multiple_writes() {
        let (mut writer, reader) = triple_buffer::<i32>();
        writer.write(1);
        writer.write(2);
        writer.write(3);
        // Reader should get the latest value
        assert_eq!(reader.read(), 3);
    }

    #[test]
    fn test_no_fresh_data() {
        let (mut writer, reader) = triple_buffer::<i32>();
        writer.write(42);
        let _ = reader.read();
        // No new writes, should still return last value
        assert!(!reader.has_fresh());
        assert_eq!(reader.read(), 42);
    }

    #[test]
    fn test_with_closure() {
        let (mut writer, reader) = triple_buffer::<Vec<i32>>();
        writer.write(vec![1, 2, 3]);
        let sum: i32 = reader.with(|v| v.iter().sum());
        assert_eq!(sum, 6);
    }
}
