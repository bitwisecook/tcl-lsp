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

const obj = @import("../valtypes/tcl_obj.zig");
const builtin = @import("builtin");

// Test builds replace the host bridge with a Zig-side stub that
// panics if invoked: the unit-test binaries reach
// ``tcl_dispatch`` transitively (every ``tcl_runtime`` import
// drags it in via the command table) but should never exercise
// the cross-context call path.  Returning 0 silently would look
// like a successful empty Tcl result — and the contract above
// already documents that 0 is a valid result, so the host bridge
// must trap on dispatch failure.  Mirror that here so any test
// that accidentally walks into this path surfaces the missing
// wiring instead of producing a false-positive pass.  Production
// embedders continue to provide the real
// ``env::call_compiled_proc``.
const env_call_compiled = if (builtin.is_test) struct {
    pub fn call_compiled_proc(
        name_ptr: i32,
        name_len: i32,
        argv_ptr: i32,
        argc: i32,
    ) i32 {
        _ = .{ name_ptr, name_len, argv_ptr, argc };
        @panic("test build: call_compiled_proc invoked — proc dispatch is not wired in unit tests");
    }
} else struct {
    pub extern "env" fn call_compiled_proc(
        name_ptr: i32,
        name_len: i32,
        argv_ptr: i32,
        argc: i32,
    ) i32;
};
const call_compiled_proc = env_call_compiled.call_compiled_proc;

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
    const procs = @import("../interp/tcl_procs.zig");
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

    // Compiled-fn arity we have to satisfy.  ``proc_get_n_params`` /
    // ``proc_get_n_required`` read i32 fields from the Command record;
    // if the bucket has been corrupted (e.g. by an over-aggressive
    // free under cascaded trace re-entry — Stream 1 trace.test) those
    // reads can return negative values, which an ``@intCast`` to u32
    // would convert to a panic in ReleaseSafe.  Treat negatives as
    // "no arity constraint" rather than aborting the whole module so
    // the surrounding ``catch`` (or the trace-handler boundary) can
    // surface the actual proc-call problem.
    const np_raw = procs.proc_get_n_params(bucket);
    const n_params: u32 = if (np_raw <= 0) 0 else @intCast(np_raw);
    const has_args_tail: bool = procs.proc_get_args_tail(bucket) != 0;

    // Arity check: if the caller supplied fewer arguments than the
    // minimum required (params with no declared default), raise
    // "wrong # args" before calling the compiled WASM function.
    // This mirrors what the Tcl interpreter does for interpreted procs
    // and lets ``catch {proc_missing_required_arg}`` return 1.
    const nr_raw = procs.proc_get_n_required(bucket);
    const n_required: u32 = if (nr_raw <= 0) 0 else @intCast(nr_raw);
    if (n_required > 0 and given < n_required) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        // name_ptr / name_len are raw bytes (not a TclObj).
        const proc_len: u32 = @bitCast(name_len);
        const prefix: []const u8 = "wrong # args: should be \"";
        const suffix: []const u8 = " arg ...\"";
        const total: u32 = @as(u32, @intCast(prefix.len)) + proc_len + @as(u32, @intCast(suffix.len));
        const buf = obj.alloc(total);
        if (buf == 0) {
            catch_mod.tcl_cmd_error(0);
            return 0;
        }
        const d: [*]u8 = @ptrFromInt(buf);
        for (prefix, 0..) |b, k| d[k] = b;
        if (proc_len > 0) {
            const np: [*]const u8 = @ptrFromInt(@as(u32, @bitCast(name_ptr)));
            for (0..proc_len) |k| d[prefix.len + k] = np[k];
        }
        for (suffix, 0..) |b, k| d[prefix.len + proc_len + k] = b;
        // Issue #317: see ``build_args_list`` below.
        const msg = obj.obj_new_string_take(buf, total, total);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

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
    const argv_buf_size: u32 = argc * 4;
    // Track the tail-args list so we can release it after the
    // compiled proc returns.  ``build_args_list`` returns a fresh
    // +1 owned obj; the compiled proc's prologue retains via
    // ``frame_local_set_at`` for the ``args`` slot, then the
    // matching ``frame_pop`` releases.  Without this caller-side
    // release every ``args``-bearing compiled-proc dispatch leaked
    // one TclObj header per call.
    var tail_list: i32 = 0;
    if (argc > 0) {
        argv_buf = obj.alloc(argv_buf_size);
        if (argv_buf == 0) return 0;
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
            tail_list = build_args_list(words, fixed);
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
    // ``@bitCast`` not ``@intCast`` so a heap-grown ``argv_buf`` past
    // the 2 GiB high-bit boundary doesn't panic the whole module on
    // narrowing — the host-side ``env::call_compiled_proc`` reads
    // ``argv_ptr`` as an i32 and bitcasts back to a u32 address, so
    // the bit pattern round-trips unchanged.  Stream 1: trace.test's
    // cumulative trace re-entry pushed the heap past the boundary
    // before reaching the test summary, blowing up here on what was
    // an otherwise benign proc dispatch.
    const result = call_compiled_proc(
        name_ptr,
        name_len,
        @bitCast(argv_buf),
        @bitCast(argc),
    );
    if (tail_list != 0) obj.tcl_obj_release(tail_list);
    if (argv_buf != 0) obj.free_sized(argv_buf, argv_buf_size);
    return result;
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
    const rt = @import("../tcl_runtime.zig");
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
    if (buf == 0) return obj.obj_new_string(0, 0);
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
    // Issue #317: ``obj_new_string_take`` so the args list owns
    // ``buf`` (so its eventual release frees the slab).  The
    // older borrowing form leaked one buf per compiled-proc
    // dispatch with a tail ``args`` parameter.
    return obj.obj_new_string_take(buf, off, total);
}
