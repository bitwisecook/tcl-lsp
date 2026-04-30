// Asyncify intrinsics — Stage 2 coroutine driver.
//
// Binaryen's ``wasm-opt --asyncify`` pass instruments every function
// in the module to support stack-saving / -restoring around calls to
// the five control functions exposed here.  Before the pass runs they
// resolve as imports from the ``env`` module; the pass replaces each
// import with a call to the auto-generated internal implementation
// and re-exports it.
//
// Calling shape:
//
//   * ``asyncify_start_unwind(buf)`` — begin saving the call stack
//     into ``buf``.  Every instrumented function on the call chain
//     stores its locals into ``buf`` and returns; control returns to
//     whoever invoked the entry point.  The first 8 bytes of ``buf``
//     hold a ``(write_ptr, end_ptr)`` pair the runtime updates as it
//     saves locals; the rest is scratch.  Sizing is per-coroutine
//     and grows on demand (see :file:`sched/tcl_coro.zig`).
//
//   * ``asyncify_stop_unwind()`` — call from the entry-point caller
//     after ``asyncify_get_state()`` reports state 1 (UNWINDING) so
//     the runtime knows the unwind is complete and the next call can
//     start fresh.
//
//   * ``asyncify_start_rewind(buf)`` — re-enter the saved call stack
//     starting from ``buf``.  Subsequently invoking the same entry
//     point fast-forwards through every instrumented function back to
//     the call site of ``asyncify_start_unwind``, which then returns.
//
//   * ``asyncify_stop_rewind()`` — paired with ``start_rewind`` once
//     the stack has been fully restored.
//
//   * ``asyncify_get_state()`` returns one of:
//        0 = NORMAL    (no unwind / rewind in flight)
//        1 = UNWINDING (saving locals into the buffer)
//        2 = REWINDING (replaying locals out of the buffer)
//        3 = REWIND_DONE
//
// When the runtime is built without ``-Dasyncify=true`` these symbols
// resolve as unsatisfied imports and the linker fails.  Guard every
// call site behind ``build_options.asyncify`` so the standard build
// stays self-contained.

const build_options = @import("build_options");

pub const STATE_NORMAL: i32 = 0;
pub const STATE_UNWINDING: i32 = 1;
pub const STATE_REWINDING: i32 = 2;

// Asyncify stack data laid out at the head of the buffer:
//   bytes  0..3  : current "stack pointer" — write head while
//                  unwinding, read head while rewinding.
//   bytes  4..7  : end of the buffer (set once at allocation).
//   bytes  8..   : scratch the asyncify runtime owns.
pub const ASYNCIFY_HEADER_SIZE: u32 = 8;

/// Initial size of a freshly-allocated coroutine asyncify buffer.
/// 16 KB covers a few hundred frames worth of typical Tcl locals;
/// callers grow it on demand if the runtime reports overflow.
pub const DEFAULT_BUFFER_SIZE: u32 = 16 * 1024;

// Asyncify intrinsics — only declared as imports when the build is
// post-processed.  Default (non-asyncify) builds compile to no-op
// stubs so the runtime artefact has no dangling ``env.asyncify_*``
// imports the linker can't satisfy.

const env_imports = if (build_options.asyncify) struct {
    pub extern "env" fn asyncify_start_unwind(buf: u32) void;
    pub extern "env" fn asyncify_stop_unwind() void;
    pub extern "env" fn asyncify_start_rewind(buf: u32) void;
    pub extern "env" fn asyncify_stop_rewind() void;
    pub extern "env" fn asyncify_get_state() i32;
} else struct {
    pub fn asyncify_start_unwind(buf: u32) void { _ = buf; }
    pub fn asyncify_stop_unwind() void {}
    pub fn asyncify_start_rewind(buf: u32) void { _ = buf; }
    pub fn asyncify_stop_rewind() void {}
    pub fn asyncify_get_state() i32 { return STATE_NORMAL; }
};

pub const asyncify_start_unwind = env_imports.asyncify_start_unwind;
pub const asyncify_stop_unwind = env_imports.asyncify_stop_unwind;
pub const asyncify_start_rewind = env_imports.asyncify_start_rewind;
pub const asyncify_stop_rewind = env_imports.asyncify_stop_rewind;
pub const asyncify_get_state = env_imports.asyncify_get_state;

/// Initialize the (start_ptr, end_ptr) header at the head of a fresh
/// asyncify buffer.  The asyncify runtime reads these on every
/// unwind / rewind to know where to write / read locals.
pub fn init_buffer(buf_addr: u32, buf_size: u32) void {
    const obj = @import("../valtypes/tcl_obj.zig");
    obj.write_i32(buf_addr, @bitCast(buf_addr + ASYNCIFY_HEADER_SIZE));
    obj.write_i32(buf_addr + 4, @bitCast(buf_addr + buf_size));
}

/// Whether this build was post-processed with wasm-opt --asyncify.
/// Coroutine command paths gate their behavior on this flag — when
/// false they fall through to the segment-based v1 driver.
pub const ENABLED: bool = build_options.asyncify;
