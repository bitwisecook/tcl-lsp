// Lazy materialisation of qualified BUILTIN namespaces.
//
// The static :data:`BUILTINS` slice in ``tcl_cmd_table.zig`` is keyed
// by fully-qualified name (e.g. ``::tcl::mathop::+``) and is queried
// directly by :func:`tcl_cmd_table.lookup`.  Those entries are *not*
// in any namespace's per-ns ``cmd_table``, so until a caller pokes at
// the qualified prefix, the namespace tree contains no record that
// ``::tcl::mathop`` (and friends) "exist".  That makes
// ``namespace import ::tcl::mathop::*`` raise ``unknown namespace``,
// ``info commands ::tcl::mathop::*`` return empty, and any other code
// that walks the namespace tree miss the BUILTIN-tier commands.
//
// :func:`materialise` plugs the gap.  Given a qualified prefix ``P``,
// it walks the BUILTINS slice once, creates the ``P`` namespace via
// ``ns_create_from_fqn``, and registers each direct child of the
// prefix (entries whose name is ``P::SIMPLE`` with no further ``::``
// in ``SIMPLE``) as a ``CMD_BUILTIN_FORWARD`` Command in that
// namespace's ``cmd_table`` via ``register_builtin_forward``.  It
// then stamps ``namespace export *`` so the populated commands are
// visible to ``namespace import``.  Idempotent: calling it twice is
// harmless because ``register_builtin_forward`` overwrites with the
// same handler address and the export-list guard skips re-stamping.
//
// This is the structural counterpart of C Tcl's ``Tcl_CreateNamespace``
// hooks at startup — there, every ``Tcl_CreateObjCommand("::tcl::mathop::+", ...)``
// implicitly populates the parent namespace.  Our static BUILTINS
// table doesn't go through that path, so we materialise on demand.

const cmd_table = @import("tcl_cmd_table.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");
const procs = @import("../interp/tcl_procs.zig");
const obj = @import("../valtypes/tcl_obj.zig");

// Static ``*`` byte stamped into newly-materialised namespaces'
// export pattern lists.  ``ns_export`` heap-copies the bytes, so the
// pointer just needs to live somewhere stable in linear memory; the
// constant data segment is the natural home.  Avoids the wasted
// ``alloc(1)`` per materialised namespace that the bump allocator
// would otherwise leak (it can't free).
const STAR_BYTE: [1]u8 = .{'*'};

/// True iff at least one BUILTIN entry's name is ``<prefix>::<simple>``
/// AND fully-qualified (``::``-prefixed) — i.e. the prefix is a real
/// namespace from the BUILTINS table's point of view.  The
/// fully-qualified filter mirrors what ``materialise`` actually
/// registers; without it, a prefix that only matched the
/// half-qualified BUILTIN spellings would produce an empty
/// materialised namespace.  Cheap O(N) over the ~few-hundred-entry
/// BUILTINS slice.
pub fn has_builtin_namespace(prefix_ptr: u32, prefix_len: u32) bool {
    if (prefix_len == 0) return false;
    const prefix: [*]const u8 = @ptrFromInt(prefix_ptr);
    for (cmd_table.entries()) |e| {
        if (e.name.len < 2 or e.name[0] != ':' or e.name[1] != ':') continue;
        if (entry_is_under_prefix(e.name, prefix, prefix_len)) return true;
    }
    return false;
}

/// Find-or-create the ``prefix`` namespace and populate its
/// ``cmd_table`` with ``CMD_BUILTIN_FORWARD`` redirects for every
/// direct-child BUILTIN entry.  Stamps ``namespace export *`` on
/// the namespace the first time around so subsequent
/// ``namespace import <prefix>::*`` finds the entries exported.
///
/// Returns the namespace handle on success, 0 if the prefix does not
/// match any BUILTIN entries (caller should fall back to the regular
/// "unknown namespace" diagnostic).
pub fn materialise(prefix_ptr: u32, prefix_len: u32) u32 {
    if (prefix_len == 0) return 0;
    if (!has_builtin_namespace(prefix_ptr, prefix_len)) return 0;

    const ns_addr = tcl_ns.ns_create_from_fqn(prefix_ptr, prefix_len);
    if (ns_addr == 0) return 0;

    const prefix: [*]const u8 = @ptrFromInt(prefix_ptr);
    for (cmd_table.entries()) |e| {
        if (!entry_is_under_prefix(e.name, prefix, prefix_len)) continue;
        // Only register entries whose name is fully-qualified
        // (``::``-prefixed).  ``cmds/tcl_mathop.zig`` also lists a
        // half-qualified spelling (``tcl::mathop::+``) for the
        // ``namespace eval ::tcl`` callers; passing that to
        // ``register_builtin_forward`` would resolve relative to
        // ``ns_current()`` and create the wrong tree (e.g.
        // ``::caller::tcl::mathop::+`` when called from inside a
        // non-root namespace).  Skipping the half-qualified shape
        // means we register each operator exactly once, anchored
        // at root via the ``::``-prefixed name.
        if (e.name.len < 2 or e.name[0] != ':' or e.name[1] != ':') continue;
        const simple_off = prefix_len + 2;
        if (entry_has_deeper_separator(e.name, simple_off)) continue;
        // Skip if some other path (a user proc, an alias, …) has
        // already populated this slot in the ns cmd_table.  Don't
        // clobber: a proc shadow is the user's deliberate override.
        const simple_ptr = @intFromPtr(e.name.ptr) + simple_off;
        const simple_len: u32 = @intCast(e.name.len - simple_off);
        const existing = tcl_ns.ns_cmd_find(ns_addr, simple_ptr, simple_len);
        if (existing != 0) {
            // ``register_builtin_forward`` rewrites flags / handler
            // unconditionally — that's the right shape when the
            // existing entry is already a forward we placed earlier
            // (idempotent re-stamp), but it would clobber a user
            // proc.  Skip the rewrite when the slot is owned by
            // something other than us.
            if (!is_builtin_forward(existing)) continue;
        }
        const handler_addr: u32 = @intCast(@intFromPtr(e.handler));
        // ``register_builtin_forward`` resolves the FQN itself via
        // ``ns_resolve_qualified_creating``, so passing the full
        // ``::tcl::mathop::+`` string lands the entry in the right
        // namespace regardless of ``ns_current``.
        const fqn_ptr: u32 = @intFromPtr(e.name.ptr);
        const fqn_len: u32 = @intCast(e.name.len);
        procs.register_builtin_forward(fqn_ptr, fqn_len, handler_addr);
    }

    // ``namespace export *`` — but only if no exports are registered
    // yet.  Re-stamping would leak (the export list never shrinks)
    // and the existing pattern set already covers ``*``.
    const ns: *const tcl_ns.Namespace = @ptrFromInt(ns_addr);
    if (ns.export_pattern_count == 0) {
        tcl_ns.ns_export(ns_addr, @intFromPtr(&STAR_BYTE[0]), 1);
    }

    return ns_addr;
}

/// True iff ``name`` is ``<prefix>::<rest>`` with non-empty ``rest``.
fn entry_is_under_prefix(name: []const u8, prefix: [*]const u8, prefix_len: u32) bool {
    if (name.len <= prefix_len + 2) return false;
    var i: u32 = 0;
    while (i < prefix_len) : (i += 1) {
        if (name[i] != prefix[i]) return false;
    }
    if (name[prefix_len] != ':' or name[prefix_len + 1] != ':') return false;
    return true;
}

/// True iff ``name[simple_off..]`` contains a further ``::`` —
/// meaning this BUILTIN belongs to a deeper namespace, not a direct
/// child of ``prefix``.
fn entry_has_deeper_separator(name: []const u8, simple_off: u32) bool {
    if (name.len < simple_off + 2) return false;
    var k: u32 = simple_off;
    while (k + 1 < name.len) : (k += 1) {
        if (name[k] == ':' and name[k + 1] == ':') return true;
    }
    return false;
}

/// True iff the existing Command at ``cmd_addr`` is a
/// ``CMD_BUILTIN_FORWARD`` we stamped earlier — safe to re-stamp.
fn is_builtin_forward(cmd_addr: u32) bool {
    const flags: u32 = @bitCast(obj.read_i32(cmd_addr + procs.OFF_FLAGS));
    return (flags & procs.CMD_BUILTIN_FORWARD) != 0;
}
