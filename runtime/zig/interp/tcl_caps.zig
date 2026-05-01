// Capability bitset gating risky commands in the WASM runtime.
//
// The runtime ships sandboxed by default — every guest call into
// :data:`CAP`-gated commands (``exec``, ``exit``, ``glob``) raises
// ``permission denied: <cmd> requires CAP_<NAME>`` through the
// standard catch-aware error path.  Embedders that have a legitimate
// need for those commands flip the matching bit by calling the
// exported :func:`tcl_set_capabilities` before they invoke
// ``tcl_eval``.
//
// The bitset is a single ``u32`` global rather than a per-interp
// field because every consumer scenario we have today is "the host
// configures the runtime once at startup, then runs untrusted Tcl
// against it".  If we ever need per-child-interp capabilities (a
// "safe" child whose parent is trusted) the storage moves onto
// ``tcl_interp_registry.Interp`` and :func:`check` takes the active
// interp as input.
//
// Wire-up:
//   - Each capability-gated handler calls
//     ``caps.check(caps.CAP_EXEC, "exec")`` (or the matching pair)
//     before performing any action.  The check raises through
//     ``stubs.tcl_stubs`` so ``catch`` observes a clean Tcl error
//     return and bare invocations land in the runtime trap path.
//   - ``tcl_set_capabilities`` is a plain WASM export.  The Python
//     test harness calls it before driving the script under test;
//     production embedders call it from their initialiser.
//
// Naming:
//   - ``CAP_FS_GLOB`` reads as "the filesystem-side glob primitive";
//     it sits under the ``FS_`` prefix because future filesystem
//     escape hatches (write outside preopens, raw fd ops) will share
//     the same scope.

const stubs = @import("../stubs/tcl_stubs.zig");
const obj = @import("../valtypes/tcl_obj.zig");

/// Bit positions inside the global capability mask.  Powers of two so
/// the bitset can be ORed / ANDed cleanly; integer values are
/// stable ABI for the host caller.
pub const CAP_EXEC: u32 = 1 << 0;
pub const CAP_EXIT: u32 = 1 << 1;
pub const CAP_FS_GLOB: u32 = 1 << 2;

/// Active capability mask.  Zero by default — every guarded handler
/// refuses until the host explicitly grants the matching bit.  The
/// global is shared across the whole runtime instance; child interps
/// inherit it.
var capabilities: u32 = 0;

/// Set the capability mask in one shot.  The host calls this before
/// ``tcl_eval`` to grant whichever subset of dangerous primitives the
/// embedding scenario actually needs.  Passing ``0`` resets to the
/// default sandboxed posture.
///
/// We deliberately overwrite rather than OR so an embedder that
/// dropped privileges via ``tcl_set_capabilities(0)`` mid-run can
/// trust that subsequent ``exec`` / ``exit`` / ``glob`` calls fail.
pub export fn tcl_set_capabilities(bits: u32) void {
    capabilities = bits;
}

/// Read the current capability mask.  Exists so the harness can
/// assert that ``tcl_set_capabilities`` round-trips and so a host
/// that wraps the runtime in additional policy can read the active
/// mask without shadowing it.
pub export fn tcl_get_capabilities() u32 {
    return capabilities;
}

/// Return ``true`` when *flag* is currently granted.  Used by
/// internal Zig callers (the capability-gated handlers) — guests
/// observe the policy through :func:`check` which also raises on
/// failure.
pub fn has(flag: u32) bool {
    return (capabilities & flag) != 0;
}

/// Verify that *flag* is set, returning ``true`` on success.  On
/// miss this raises ``permission denied: <cmd> requires CAP_<NAME>``
/// via :func:`stubs.raise` so ``catch`` integrations work — and
/// returns ``false`` so the caller can early-exit cleanly without
/// mistakenly running the underlying primitive.
///
/// *cmd_name* and *cap_name* should both be plain ASCII string
/// literals.  Building the message inline keeps the call site terse
/// (``if (!caps.check(caps.CAP_EXEC, "exec", "EXEC")) return …``).
pub fn check(flag: u32, cmd_name: []const u8, cap_name: []const u8) bool {
    if (has(flag)) return true;
    const prefix: []const u8 = "permission denied: ";
    const middle: []const u8 = " requires CAP_";
    const total: u32 = @intCast(prefix.len + cmd_name.len + middle.len + cap_name.len);
    const buf_addr: u32 = obj.alloc(total);
    const buf: [*]u8 = @ptrFromInt(buf_addr);
    var off: usize = 0;
    for (prefix) |c| {
        buf[off] = c;
        off += 1;
    }
    for (cmd_name) |c| {
        buf[off] = c;
        off += 1;
    }
    for (middle) |c| {
        buf[off] = c;
        off += 1;
    }
    for (cap_name) |c| {
        buf[off] = c;
        off += 1;
    }
    // Hand the assembled message to ``stubs.raise`` so the
    // catch-aware error path / runtime-trap formatter is the same
    // one every other capability-style refusal uses — keeps the
    // diagnostic surface consistent.
    const msg_slice = (@as([*]const u8, @ptrFromInt(buf_addr)))[0..total];
    stubs.raise(msg_slice);
    return false;
}
