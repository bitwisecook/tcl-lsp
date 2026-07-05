// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Alloc / free / double-free counters — the leak-check instrumentation
//! (`memory-management.md` MM-C). These back the T1.1 acceptance gate (a
//! balanced round-trip leaves zero residual) and the T1.3 frame/var leak tests,
//! mirroring the Zig runtime's `tcl_test_*` surface.
//!
//! **Thread-local**, not global. The production runtime is a single-threaded
//! WASM reactor, so per-thread counters are semantically identical there (one
//! thread) while making the **native `cargo test`** build correct under
//! parallel test execution: each test thread allocates, frees, and checks its
//! own counters with no cross-test interference. (Global atomics would race —
//! one test's `reset` would clobber another's in-flight count.)
//!
//! Always-on here; the eventual wasm build gates them behind a `leak-check`
//! feature so the production hot path pays nothing (MM-C).

use std::cell::Cell;

thread_local! {
    static OBJS_ALLOCED: Cell<i64> = const { Cell::new(0) };
    static OBJS_FREED: Cell<i64> = const { Cell::new(0) };
    static BUFS_ALLOCED: Cell<i64> = const { Cell::new(0) };
    static BUFS_FREED: Cell<i64> = const { Cell::new(0) };
    static DOUBLE_FREES: Cell<i64> = const { Cell::new(0) };
    static OOM: Cell<i64> = const { Cell::new(0) };
}

fn bump(key: &'static std::thread::LocalKey<Cell<i64>>) {
    key.with(|c| c.set(c.get() + 1));
}

pub(crate) fn obj_alloced() {
    bump(&OBJS_ALLOCED);
}
pub(crate) fn obj_freed() {
    bump(&OBJS_FREED);
}
pub(crate) fn buf_alloced() {
    bump(&BUFS_ALLOCED);
}
pub(crate) fn buf_freed() {
    bump(&BUFS_FREED);
}
pub(crate) fn double_free() {
    bump(&DOUBLE_FREES);
}
pub(crate) fn oom_set() {
    OOM.with(|c| c.set(1));
}

fn get(key: &'static std::thread::LocalKey<Cell<i64>>) -> i64 {
    key.with(|c| c.get())
}

/// Total `TclObj` headers allocated since the last reset (parity with the Zig
/// `tcl_test_alloc_count`).
pub fn alloc_count() -> i64 {
    get(&OBJS_ALLOCED)
}

/// Number of `Tcl_DecrRefCount` calls on an already-zero-refcount object — a
/// contract violation. Must be zero after a correct run.
pub fn double_free_count() -> i64 {
    get(&DOUBLE_FREES)
}

/// Whether the allocator has hit OOM since the last reset.
pub fn oom() -> bool {
    get(&OOM) != 0
}

/// Live (unfreed) `TclObj` headers.
pub fn live_objs() -> i64 {
    get(&OBJS_ALLOCED) - get(&OBJS_FREED)
}

/// Live (unfreed) owned string buffers.
pub fn live_bufs() -> i64 {
    get(&BUFS_ALLOCED) - get(&BUFS_FREED)
}

/// Residual = live headers + live buffers. Zero ⇒ leak-free (`tcl_test_finalize`).
pub fn finalize() -> i64 {
    live_objs() + live_bufs()
}

/// Reset all counters (start of a measured run — `tcl_test_reset_counters`).
pub fn reset() {
    for key in [
        &OBJS_ALLOCED,
        &OBJS_FREED,
        &BUFS_ALLOCED,
        &BUFS_FREED,
        &DOUBLE_FREES,
        &OOM,
    ] {
        key.with(|c| c.set(0));
    }
}
