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
// result).
//
// Error handling: ``0`` is a valid TclObj sentinel meaning "empty
// string" / "null result" — it is NOT an error signal.  Embedders
// that can't dispatch (unknown name, memory unmapped, compiled
// proc traps) MUST raise a wasmtime trap rather than return 0,
// otherwise real failures look like successful empty returns.  The
// Python harness in tests/test_wasm_real_tcl.py does this by
// letting ``func(store, *args)`` propagate its trap and by raising
// ``RuntimeError`` for the other failure modes.
//
// Must be declared by the embedder before the runtime module is
// instantiated.  Tests that don't exercise cross-context calls
// should still provide it — raising from the callback is fine.

const obj = @import("tcl_obj.zig");

extern "env" fn call_compiled_proc(
    name_ptr: i32,
    name_len: i32,
    argv_ptr: i32,
    argc: i32,
) i32;

/// Dispatch a proc's associated compiled proc via the host bridge.
///
/// ``bucket`` is the ``proc_lookup`` result for the resolved name
/// — using the bucket's stored name (fully qualified) rather than
/// ``words[0]`` ensures the embedder looks up the right WASM
/// export when the caller invoked the proc via an unqualified
/// name resolved through namespace-path search (e.g. calling
/// ``AcceptAll`` from inside ``::tcltest`` resolves the bucket
/// for ``::tcltest::AcceptAll`` — the compiled module exports it
/// by that qualified name).  Packs ``words[1..]`` into a
/// contiguous i32 buffer and hands (name_ptr, name_len, argv_ptr,
/// argc) to ``call_compiled_proc``.
pub fn dispatch(bucket: i32, words: []const i32) i32 {
    const procs = @import("tcl_procs.zig");
    const name_ptr = procs.proc_get_name_ptr(bucket);
    const name_len = procs.proc_get_name_len(bucket);
    // Pack argv[1..] (the "real" args, not the command name) into
    // a fresh i32 array.  Each TclObj pointer is 4 bytes; write
    // them little-endian to match the WASM memory layout.
    const argc: u32 = if (words.len == 0) 0 else @intCast(words.len - 1);
    var argv_buf: u32 = 0;
    if (argc > 0) {
        argv_buf = obj.alloc(argc * 4);
        var i: u32 = 0;
        while (i < argc) : (i += 1) {
            obj.write_i32(argv_buf + i * 4, words[i + 1]);
        }
    }
    return call_compiled_proc(
        name_ptr,
        name_len,
        @intCast(argv_buf),
        @intCast(argc),
    );
}
