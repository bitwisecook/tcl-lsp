// Host bridge for cross-context compiled-proc dispatch.
//
// When the interpreter walks a Tcl-source proc body (e.g. a
// dynamically-registered proc created via ``proc $varName {body}``)
// and encounters a command that resolves to a *compiled* WASM proc,
// pure WASM can't call it: the runtime is one module, the compiled
// user code is another, and WASM 1.0 has no direct way to call
// across modules without funcref tables.
//
// We sidestep this by declaring an imported host callback
// ``env.call_compiled_proc`` and having the Python harness (or any
// other embedder) dispatch by proc name.  The embedder sees both
// modules and knows how to turn a name into a wasmtime callable.
//
// Signature:
//   call_compiled_proc(name_ptr, name_len, argv_ptr, argc) -> i32
//
// ``name_ptr``/``name_len``: address + length of the proc's
// qualified name in the runtime's linear memory.
// ``argv_ptr``: address of a contiguous array of ``argc`` i32
// TclObj pointers — the arguments to pass to the compiled proc,
// *not* including the command name (argv[0] is the first real arg).
// ``argc``: number of args.  Returns an i32 TclObj (the proc's
// result) or 0 if the embedder couldn't dispatch.
//
// Must be declared by the embedder before the runtime module is
// instantiated.  Tests that don't exercise cross-context calls can
// provide a stub that returns 0.

const obj = @import("tcl_obj.zig");

extern "env" fn call_compiled_proc(
    name_ptr: i32,
    name_len: i32,
    argv_ptr: i32,
    argc: i32,
) i32;

/// Dispatch ``words[0]``'s associated compiled proc via the host
/// bridge.  Packs ``words[1..]`` into a contiguous i32 buffer and
/// hands the pointer + length to ``call_compiled_proc``.  Returns
/// the TclObj result the embedder handed back.
pub fn dispatch(words: []const i32) i32 {
    if (words.len == 0) return 0;
    const name = obj.obj_ensure_string(words[0]);
    // Pack argv[1..] (the "real" args, not the command name) into
    // a fresh i32 array.  Each TclObj pointer is 4 bytes; write
    // them little-endian to match the WASM memory layout.
    const argc: u32 = @intCast(words.len - 1);
    var argv_buf: u32 = 0;
    if (argc > 0) {
        argv_buf = obj.alloc(argc * 4);
        var i: u32 = 0;
        while (i < argc) : (i += 1) {
            obj.write_i32(argv_buf + i * 4, words[i + 1]);
        }
    }
    return call_compiled_proc(
        @intCast(name.ptr),
        @intCast(name.len),
        @intCast(argv_buf),
        @intCast(argc),
    );
}
