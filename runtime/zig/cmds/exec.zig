// ``exec`` — run a subprocess via the host-imported ``host_spawn``.
//
// WASI's ``wasi_snapshot_preview1`` only exposes ``proc_exit``, not
// ``proc_spawn``, so the runtime cannot fork its own children.  To
// support ``exec`` at all we declare an additional host import,
// ``env.host_spawn``, that the embedder wires to whatever subprocess
// strategy fits the deployment (a real ``posix_spawn`` on the
// server, a denylist-checking shim in tests, a fully-virtual
// in-memory dispatcher for sandboxed CTF runs).  Embedders that
// don't grant :data:`caps.CAP_EXEC` never trigger the call —
// :func:`caps.check` raises ``permission denied`` first — so the
// import is reached only on opt-in.
//
// The argv is serialised to a NUL-separated UTF-8 buffer in linear
// memory; *argv_len* is the byte length including the trailing NUL
// after the last argument.  This shape sidesteps allocating an
// array-of-pointers across the WASM/host boundary (the host walks
// the buffer and splits on NUL).  ``stdin`` follows the same shape
// — empty span (ptr=0, len=0) means "no stdin".
//
// The host returns a TclObj handle directly: a string TclObj when
// the exit status was 0, or 0 (zero) when the spawn / process
// failed.  On failure the host is responsible for routing a Tcl
// error into the catch path before returning; that mirrors the
// ``stubs.raise`` contract every other command uses, so ``catch``
// integrations require no special-casing here.
//
// Form support:
//   ``exec arg ?arg ...?``   — every argument becomes one argv
//                              entry; *all* the switches Tcl
//                              accepts (-ignorestderr / -keepnewline
//                              / --) are passed through to the host
//                              for it to interpret.  Leaving switch
//                              parsing on the host side means we
//                              don't have to grow the runtime to
//                              match every Tcl 8.4–9.0 behaviour
//                              the spec page calls out.

const reg = @import("../dispatch/tcl_cmd_registry.zig");
const obj = @import("../valtypes/tcl_obj.zig");
const stubs = @import("../stubs/tcl_stubs.zig");
const caps = @import("../interp/tcl_caps.zig");

/// Host-imported subprocess primitive.  Returns the result TclObj
/// handle on success or 0 on failure (with the host having already
/// raised a Tcl error via the catch machinery before returning).
///
/// Embedders MUST satisfy this import even if they never grant
/// ``CAP_EXEC`` — the WASM linker rejects the module otherwise.  A
/// stub that raises ``host_spawn: not configured`` is the
/// recommended posture for sandboxed deployments.
extern "env" fn host_spawn(
    argv_ptr: u32,
    argv_len: u32,
    stdin_ptr: u32,
    stdin_len: u32,
) i32;

fn eval_exec(words: []const i32) i32 {
    if (!caps.check(caps.CAP_EXEC, "exec", "EXEC")) return 0;
    if (words.len < 2) {
        stubs.raise("wrong # args: should be \"exec ?switches? arg ?arg ...?\"");
        return 0;
    }

    // Compute the serialised buffer size up front so we can allocate
    // a single span on the bump allocator and avoid a growth loop.
    // Embedded NUL bytes inside an arg would silently truncate the
    // host-side split (NUL is our argv separator), so we reject
    // them with a Tcl error before any allocation — matches the
    // shell semantics every Unix kernel enforces and gives ``catch``
    // a clean diagnostic.
    var total: u32 = 0;
    for (words[1..]) |w| {
        const s = obj.obj_ensure_string(w);
        if (s.len > 0) {
            const src: [*]const u8 = @ptrFromInt(s.ptr);
            for (0..s.len) |i| {
                if (src[i] == 0) {
                    stubs.raise("exec: argument contains embedded NUL byte");
                    return 0;
                }
            }
        }
        total += s.len + 1; // +1 for the inter-arg NUL separator
    }
    if (total == 0) {
        stubs.raise("exec: no arguments");
        return 0;
    }

    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: u32 = 0;
    for (words[1..]) |w| {
        const s = obj.obj_ensure_string(w);
        if (s.len > 0) {
            const src: [*]const u8 = @ptrFromInt(s.ptr);
            for (0..s.len) |i| buf[off + i] = src[i];
            off += s.len;
        }
        buf[off] = 0;
        off += 1;
    }

    return host_spawn(buf_addr, total, 0, 0);
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "exec", .arity_min = 1, .arity_max = null, .handler = &eval_exec },
};

// No ``pub export fn tcl_cmd_exec`` — the Python registry
// (``core/commands/registry/tcl/exec_.py``) deliberately omits a
// ``wasm_runtime_import`` because the codegen's single-arg
// truncation would silently drop trailing argv words on a multi-arg
// ``exec``.  Every call routes through the eval-fallback into
// :data:`registrations` above, which gives the BUILTIN handler full
// control over argv assembly and capability enforcement.
