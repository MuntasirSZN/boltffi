//! Dual-path Dart callback dispatch.
//!
//! Same thread: call the `fromFunction` pointer.
//! Other thread: post a `NativeCallable.listener` and block on a [`Gate`].
//! The isolate thread is never blocked on a gate.
//!
//! The `u64` handle is a pointer to a generated `Hooks` struct whose first
//! field is [`HooksHeader`].

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};

thread_local! {
    static THREAD_KEY: u8 = const { 0 };
    /// Sync-export depth on *this* thread. Foreign waiters only panic when
    /// some other thread holds a non-zero depth (the blocked isolate).
    static LOCAL_SYNC_FFI_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// Process-wide count of in-flight synchronous Dart→Rust FFI entries.
static SYNC_FFI_DEPTH: AtomicUsize = AtomicUsize::new(0);

/// Marks the duration of a sync BoltFFI export entered from the Dart isolate.
pub struct SyncFfiScope;

impl SyncFfiScope {
    #[inline]
    pub fn enter() -> Self {
        LOCAL_SYNC_FFI_DEPTH.with(|depth| depth.set(depth.get() + 1));
        SYNC_FFI_DEPTH.fetch_add(1, Ordering::Relaxed);
        Self
    }
}

impl Drop for SyncFfiScope {
    fn drop(&mut self) {
        LOCAL_SYNC_FFI_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        SYNC_FFI_DEPTH.fetch_sub(1, Ordering::Relaxed);
    }
}

#[inline]
fn foreign_wait_blocked_by_sync_ffi() -> bool {
    let global = SYNC_FFI_DEPTH.load(Ordering::Relaxed);
    if global == 0 {
        return false;
    }
    // The isolate thread that entered the export is still inside it; a
    // worker on this thread is not that isolate (local depth is 0).
    LOCAL_SYNC_FFI_DEPTH.with(|depth| depth.get() == 0)
}

fn current_thread_key() -> usize {
    THREAD_KEY.with(|cell| cell as *const u8 as usize)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum CallStatus {
    Ok = 0,
    Error = 1,
    Cancelled = 2,
}

enum GateOutcome {
    Pending,
    Done(CallStatus),
}

struct OutFree {
    ptr: *mut c_void,
    free: unsafe fn(*mut c_void),
}

unsafe impl Send for OutFree {}

pub struct Gate {
    outcome: Mutex<GateOutcome>,
    cvar: Condvar,
    out_free: Mutex<Option<OutFree>>,
}

impl Gate {
    fn resolve(&self, status: CallStatus) {
        let mut guard = self.outcome.lock().unwrap();
        if matches!(*guard, GateOutcome::Pending) {
            *guard = GateOutcome::Done(status);
            self.cvar.notify_all();
        }
    }

    pub fn wait(&self) -> CallStatus {
        // Isolate blocked in a sync export cannot drain NativeCallable.listener
        // posts, so a foreign wait would hang forever.
        if foreign_wait_blocked_by_sync_ffi() {
            panic!(
                "Dart isolate is blocked inside a synchronous BoltFFI call; \
                 a worker-thread callback cannot complete until that call returns. \
                 Avoid join/blocking on foreign-thread callbacks from sync exports \
                 (use an async export, or do the wait after returning to Dart)."
            );
        }
        let mut guard = self.outcome.lock().unwrap();
        loop {
            match &*guard {
                GateOutcome::Pending => guard = self.cvar.wait(guard).unwrap(),
                GateOutcome::Done(status) => return *status,
            }
        }
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let slot = match self.out_free.get_mut() {
            Ok(guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(out) = slot {
            unsafe { (out.free)(out.ptr) };
        }
    }
}

pub struct PendingGate {
    gate: Arc<Gate>,
    raw: *mut c_void,
}

impl PendingGate {
    pub fn raw(&self) -> *mut c_void {
        self.raw
    }

    pub fn wait(&self) -> CallStatus {
        self.gate.wait()
    }

    /// # Safety
    /// `ptr` must come from [`Box::into_raw`] and must not be freed elsewhere.
    pub unsafe fn own_out_ptr<T>(&self, ptr: *mut T) {
        unsafe fn free_box<T>(p: *mut c_void) {
            drop(unsafe { Box::from_raw(p as *mut T) });
        }
        *self.gate.out_free.lock().unwrap() = Some(OutFree {
            ptr: ptr as *mut c_void,
            free: free_box::<T>,
        });
    }

    /// Disarm drop-time free so the caller can `Box::from_raw` the slot.
    pub fn disarm_out(&self) {
        *self.gate.out_free.lock().unwrap() = None;
    }
}

struct HeaderState {
    alive: bool,
    outstanding: Vec<Weak<Gate>>,
}

/// First field of every generated `Hooks`. Handle is `*const Hooks as u64`.
/// `repr(C)` keeps `owner` at offset 0.
#[repr(C)]
pub struct HooksHeader {
    owner: usize,
    state: Mutex<HeaderState>,
}

impl Default for HooksHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl HooksHeader {
    pub fn new() -> Self {
        Self {
            owner: current_thread_key(),
            state: Mutex::new(HeaderState {
                alive: true,
                outstanding: Vec::new(),
            }),
        }
    }

    #[inline]
    pub fn is_owner(&self) -> bool {
        self.owner == current_thread_key()
    }

    pub fn create_gate(&self) -> Option<PendingGate> {
        let gate = Arc::new(Gate {
            outcome: Mutex::new(GateOutcome::Pending),
            cvar: Condvar::new(),
            out_free: Mutex::new(None),
        });
        let mut state = self.state.lock().unwrap();
        if !state.alive {
            return None;
        }
        state.outstanding.retain(|weak| weak.strong_count() > 0);
        state.outstanding.push(Arc::downgrade(&gate));
        drop(state);
        let raw = Arc::into_raw(gate.clone()) as *mut c_void;
        Some(PendingGate { gate, raw })
    }

    pub fn destroy(&self) {
        let mut state = self.state.lock().unwrap();
        state.alive = false;
        for weak in state.outstanding.iter() {
            if let Some(gate) = weak.upgrade() {
                gate.resolve(CallStatus::Cancelled);
            }
        }
    }

    pub fn outstanding(&self) -> usize {
        let mut state = self.state.lock().unwrap();
        state.outstanding.retain(|weak| weak.strong_count() > 0);
        state.outstanding.len()
    }

    /// Cancel in-flight waiters. Does not block: Dart still owns the extra
    /// gate `Arc` until `signal_gate_*`, and that listener must be allowed
    /// to run on this isolate.
    pub fn shutdown(&self) {
        self.destroy();
    }
}

/// # Safety
/// `gate_ptr` is a [`PendingGate::raw`] pointer, used exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn signal_gate_ok(gate_ptr: *mut c_void) {
    let gate = unsafe { Arc::from_raw(gate_ptr as *const Gate) };
    gate.resolve(CallStatus::Ok);
}

/// # Safety
/// Same as [`signal_gate_ok`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn signal_gate_error(gate_ptr: *mut c_void) {
    let gate = unsafe { Arc::from_raw(gate_ptr as *const Gate) };
    gate.resolve(CallStatus::Error);
}

/// First-poll continuation. Safe on any thread so a foreign wake before
/// Dart installs the isolate listener cannot abort.
#[unsafe(no_mangle)]
pub extern "C" fn poll_continuation_noop(_data: u64, _status: i8) {}

/// Cast a callback handle to its leading [`HooksHeader`].
///
/// # Safety
/// `handle` must be 0 or a pointer to a generated `Hooks` whose first
/// field is [`HooksHeader`].
#[inline]
pub unsafe fn header_from_handle<'a>(handle: u64) -> Option<&'a HooksHeader> {
    if handle == 0 {
        return None;
    }
    Some(unsafe { &*(handle as *const HooksHeader) })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that touch `Gate::wait` or `SyncFfiScope` so a
    /// held sync-export depth cannot race with unrelated waiters.
    static WAIT_TESTS: Mutex<()> = Mutex::new(());

    #[test]
    fn owner_thread_matches_constructor() {
        let header = HooksHeader::new();
        assert!(header.is_owner());
        let ok = std::thread::spawn(move || header.is_owner())
            .join()
            .unwrap();
        assert!(!ok);
    }

    #[test]
    fn gate_round_trip() {
        let _lock = WAIT_TESTS.lock().unwrap();
        let header = HooksHeader::new();
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let gate = header.create_gate().unwrap();
            tx.send(gate.raw() as usize).unwrap();
            gate.wait()
        });
        let addr = rx.recv().unwrap();
        unsafe { signal_gate_ok(addr as *mut c_void) };
        assert_eq!(waiter.join().unwrap(), CallStatus::Ok);
    }

    #[test]
    fn wait_panics_while_sync_ffi_active_on_another_thread() {
        let _lock = WAIT_TESTS.lock().unwrap();
        let _scope = SyncFfiScope::enter();
        let header = Arc::new(HooksHeader::new());
        // Worker (local depth 0) waits while this thread holds the sync export.
        let err = {
            let header = Arc::clone(&header);
            std::thread::spawn(move || {
                let gate = header.create_gate().unwrap();
                let _ = gate.wait();
            })
            .join()
            .expect_err("worker must panic instead of deadlocking")
        };
        let message = err
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| err.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("");
        assert!(
            message.contains("blocked inside a synchronous BoltFFI call"),
            "unexpected panic payload: {message}"
        );
    }

    #[test]
    fn destroy_cancels_waiters() {
        let _lock = WAIT_TESTS.lock().unwrap();
        let header = Arc::new(HooksHeader::new());
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = {
            let header = Arc::clone(&header);
            std::thread::spawn(move || {
                let gate = header.create_gate().unwrap();
                tx.send(gate.raw() as usize).unwrap();
                gate.wait()
            })
        };
        let gate_addr = rx.recv().unwrap();
        header.destroy();
        assert_eq!(waiter.join().unwrap(), CallStatus::Cancelled);
        // `create_gate` hands Dart an extra Arc via `into_raw`. Cancel
        // unblocks the waiter; the listener (or this test) must still
        // `signal_gate_*` or that Arc leaks.
        unsafe { signal_gate_ok(gate_addr as *mut c_void) };
    }

    #[test]
    fn cancelled_out_slot_stays_alive() {
        let _lock = WAIT_TESTS.lock().unwrap();
        let header = Arc::new(HooksHeader::new());
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter = {
            let header = Arc::clone(&header);
            std::thread::spawn(move || {
                let gate = header.create_gate().unwrap();
                let out = Box::into_raw(Box::new(0u64));
                unsafe { gate.own_out_ptr(out) };
                tx.send((gate.raw() as usize, out as usize)).unwrap();
                gate.wait()
            })
        };
        let (gate_addr, out_addr) = rx.recv().unwrap();
        header.destroy();
        assert_eq!(waiter.join().unwrap(), CallStatus::Cancelled);
        unsafe {
            *(out_addr as *mut u64) = 42;
            signal_gate_ok(gate_addr as *mut c_void);
        }
    }

    #[test]
    fn dead_header_refuses_new_gates() {
        let header = HooksHeader::new();
        header.destroy();
        assert!(header.create_gate().is_none());
    }

    #[test]
    fn poll_continuation_noop_is_callable_off_owner_thread() {
        std::thread::spawn(|| poll_continuation_noop(0, 1))
            .join()
            .unwrap();
    }

    #[test]
    fn shutdown_does_not_wait_for_dart_arc() {
        let header = HooksHeader::new();
        let gate = header.create_gate().unwrap();
        header.shutdown();
        assert!(header.outstanding() > 0);
        unsafe { signal_gate_ok(gate.raw()) };
        drop(gate);
        assert_eq!(header.outstanding(), 0);
    }
}
