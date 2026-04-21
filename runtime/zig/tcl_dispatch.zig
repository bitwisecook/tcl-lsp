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
///
/// Variadic ``args`` tail: when the proc declares its last
/// parameter as ``args`` (Tcl's special variadic marker), the
/// compiled function still has a fixed n_params-arity signature,
/// but Tcl semantics say ``args`` receives all excess call
/// arguments as a list.  We reconcile by packing
/// ``words[n_params..]`` into a space-joined list TclObj and
/// substituting it as the last element of the argv buffer, so
/// the compiled function always sees exactly ``n_params`` i32
/// pointers.
pub fn dispatch(bucket: i32, words: []const i32) i32 {
    const procs = @import("tcl_procs.zig");
    // Prefer the sidecar export name (set at registration time for
    // compiled procs).  If the Command was later ``rename``d, the
    // live name slot now reflects the user-visible identity — but
    // the host embedder still exports the WASM function under its
    // original name, so the sidecar is what lookups need.  Falls
    // through to the live slot when the sidecar is absent (e.g.
    // procs registered before the sidecar was introduced or
    // interpreted procs reached via this path).
    const export_name = procs.proc_get_export_name(@bitCast(bucket));
    const name_ptr: i32 = if (export_name.ptr != 0)
        @bitCast(export_name.ptr)
    else
        procs.proc_get_name_ptr(bucket);
    const name_len: i32 = if (export_name.ptr != 0)
        @bitCast(export_name.len)
    else
        procs.proc_get_name_len(bucket);

    // ``words[0]`` is the command name; actual arguments start at
    // index 1.  ``given`` = number of supplied call arguments.
    const given: u32 = if (words.len == 0) 0 else @intCast(words.len - 1);

    // Compiled-fn arity we have to satisfy.
    const n_params: u32 = @intCast(procs.proc_get_n_params(bucket));
    const has_args_tail: bool = procs.proc_get_args_tail(bucket) != 0;

    // Effective argc we pass to the host bridge.  The compiled
    // proc's WASM signature has fixed ``n_params`` arity, so we
    // always pass exactly ``n_params`` values regardless of how
    // many the caller supplied — WASM's arity check rejects
    // mismatches as a trap.  Missing slots (``given < n_params``)
    // get a null TclObj sentinel (0), which the proc's declared-
    // default bindings can substitute for.
    //
    // This padding matters when an *interpreted* call into a
    // compiled proc (via eval_proc_call → dispatch) doesn't know
    // about the callee's defaults — e.g. tcltest's
    // ``::tcltest::Configure`` with optional trailing options —
    // whereas the compiled-side ``_emit_cmd_proc_call`` already
    // pads defaults at the call site.
    const argc: u32 = n_params;

    var argv_buf: u32 = 0;
    if (argc > 0) {
        argv_buf = obj.alloc(argc * 4);
        if (has_args_tail) {
            // How many leading words map to non-``args`` params
            // (equivalently: how many positional slots we copy
            // verbatim before building the list).
            const fixed: u32 = if (n_params == 0) 0 else n_params - 1;
            var i: u32 = 0;
            while (i < fixed) : (i += 1) {
                const val: i32 = if (i < given) words[i + 1] else 0;
                obj.write_i32(argv_buf + i * 4, val);
            }
            // Remaining call args → join into a single list-shaped
            // TclObj.  Empty list when callee received ``given <=
            // fixed`` (Tcl semantics: ``args`` becomes ``{}``).
            const tail_list = build_args_list(words, fixed);
            obj.write_i32(argv_buf + fixed * 4, tail_list);
        } else {
            // Pad with 0 when caller supplied fewer args than the
            // compiled proc declares.  Excess args beyond
            // n_params are silently dropped — that matches
            // ``_emit_cmd_proc_call``'s truncation and lets procs
            // reached via ``[info level 0]`` / interp fallback
            // survive arity surprises.
            var i: u32 = 0;
            while (i < argc) : (i += 1) {
                const val: i32 = if (i < given) words[i + 1] else 0;
                obj.write_i32(argv_buf + i * 4, val);
            }
        }
    }
    return call_compiled_proc(
        name_ptr,
        name_len,
        @intCast(argv_buf),
        @intCast(argc),
    );
}

/// Build a Tcl list TclObj from ``words[fixed+1 ..]`` (skipping
/// the command name at index 0 and the ``fixed`` positional args).
/// Each element is individually list-element-quoted via the shared
/// :func:`obj.list_elem_quote` / :func:`obj.list_elem_quote_nth`
/// helpers so that ``llength`` / ``lindex`` / ``foreach`` see the
/// correct element boundaries.  Using the canonical helpers keeps
/// this path consistent with the Python compiler, the interpreter's
/// ``list`` / ``args`` collection, and ``lappend`` — a previous
/// stand-alone implementation here treated ``\\{`` as depth-
/// changing and escaped embedded newlines as literal ``\\<LF>``
/// sequences, which Tcl's script parser then folds back into
/// spaces — so a braced proc body ``{set x "a\\{"; lappend x abc}``
/// arrived at ``[lindex $args 0]`` with all its newlines replaced.
fn build_args_list(words: []const i32, fixed: u32) i32 {
    const start: u32 = fixed + 1;
    if (words.len <= start) {
        return obj.obj_new_string(0, 0);
    }
    const rt = @import("tcl_runtime.zig");
    _ = rt; // referenced indirectly via obj.obj_ensure_string
    // Worst-case expansion per element: 2x + 2 (full escape mode)
    // plus one separator byte per gap.
    var total: u32 = 0;
    var i: u32 = start;
    while (i < words.len) : (i += 1) {
        const s = obj.obj_ensure_string(words[i]);
        total += s.len * 2 + 2;
        if (i > start) total += 1;
    }
    if (total == 0) return obj.obj_new_string(0, 0);
    const buf = obj.alloc(total);
    var off: u32 = 0;
    i = start;
    while (i < words.len) : (i += 1) {
        if (i > start) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        const s = obj.obj_ensure_string(words[i]);
        if (i == start) {
            off = obj.list_elem_quote(buf, off, s.ptr, s.len);
        } else {
            off = obj.list_elem_quote_nth(buf, off, s.ptr, s.len);
        }
    }
    return obj.obj_new_string(@intCast(buf), @intCast(off));
}
