//! Alloc / free / double-free counters — the leak-check instrumentation
//! (`memory-management.md` MM-C). These back the `tcl_test_*` exports the
//! sweep harness (`scripts/leak_sweep.py`) reads, and the T1.1 acceptance
//! gate (a balanced round-trip leaves zero residual).
//!
//! Mirrors the Zig runtime's `tcl_test_alloc_count` / `tcl_test_double_free_count`
//! / `tcl_test_reset_counters` / `tcl_test_finalize` surface. Always-on here
//! (the native test build); the eventual wasm build gates them behind a
//! `leak-check` feature so the production hot path pays nothing (MM-C).

use core::sync::atomic::{AtomicI64, Ordering};

// `Relaxed` is sufficient: the runtime is single-threaded (one reactor), and
// these are pure event counters with no ordering relationship to obj data.
const ORD: Ordering = Ordering::Relaxed;

static OBJS_ALLOCED: AtomicI64 = AtomicI64::new(0);
static OBJS_FREED: AtomicI64 = AtomicI64::new(0);
static BUFS_ALLOCED: AtomicI64 = AtomicI64::new(0);
static BUFS_FREED: AtomicI64 = AtomicI64::new(0);
static DOUBLE_FREES: AtomicI64 = AtomicI64::new(0);
static OOM: AtomicI64 = AtomicI64::new(0);

pub(crate) fn obj_alloced() {
    OBJS_ALLOCED.fetch_add(1, ORD);
}
pub(crate) fn obj_freed() {
    OBJS_FREED.fetch_add(1, ORD);
}
pub(crate) fn buf_alloced() {
    BUFS_ALLOCED.fetch_add(1, ORD);
}
pub(crate) fn buf_freed() {
    BUFS_FREED.fetch_add(1, ORD);
}
pub(crate) fn double_free() {
    DOUBLE_FREES.fetch_add(1, ORD);
}
pub(crate) fn oom_set() {
    OOM.store(1, ORD);
}

/// Total `TclObj` headers allocated since the last reset (parity with the Zig
/// `tcl_test_alloc_count`).
pub fn alloc_count() -> i64 {
    OBJS_ALLOCED.load(ORD)
}

/// Number of `Tcl_DecrRefCount` calls on an already-zero-refcount object — a
/// contract violation. Must be zero after a correct run.
pub fn double_free_count() -> i64 {
    DOUBLE_FREES.load(ORD)
}

/// Whether the allocator has hit OOM since the last reset.
pub fn oom() -> bool {
    OOM.load(ORD) != 0
}

/// Live (unfreed) `TclObj` headers.
pub fn live_objs() -> i64 {
    OBJS_ALLOCED.load(ORD) - OBJS_FREED.load(ORD)
}

/// Live (unfreed) owned string buffers.
pub fn live_bufs() -> i64 {
    BUFS_ALLOCED.load(ORD) - BUFS_FREED.load(ORD)
}

/// Residual = live headers + live buffers. Zero ⇒ leak-free (`tcl_test_finalize`).
pub fn finalize() -> i64 {
    live_objs() + live_bufs()
}

/// Reset all counters (start of a measured run — `tcl_test_reset_counters`).
pub fn reset() {
    for c in [
        &OBJS_ALLOCED,
        &OBJS_FREED,
        &BUFS_ALLOCED,
        &BUFS_FREED,
        &DOUBLE_FREES,
        &OOM,
    ] {
        c.store(0, ORD);
    }
}
