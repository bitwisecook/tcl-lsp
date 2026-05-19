// Command renaming — ``rename oldName newName``.
//
// Mirror of Tcl 9's ``TclRenameCommand`` in ``tclBasic.c``.  Handles:
//
// * Simple same-ns rename (R1): update the ``cmd_table`` key + patch
//   the Command's own FQN slot.
// * Cross-namespace rename (R2): move the Command entry from one ns's
//   ``cmd_table`` to another, materialising the target ns if needed.
// * Renaming an imported redirect (R3): affects only the redirect in
//   its home ns; the source ``ImportedCmdData`` is untouched.
// * Renaming a source command that has importers (R4): the redirects
//   keep working via pointer (``ImportedCmdData.real_cmd`` is the
//   terminal address), so no cascade rewrite is needed — just flush
//   the proc-lookup LRU so importers don't serve stale cached
//   entries.
// * Rename-to-empty deletion (R5): detach the entry from its
//   ``cmd_table`` and deactivate any redirects that imported it
//   (analogous to ``ns_forget`` but driven from the source side).
//
// The hash table primitive lacks tombstones, so we use the same
// "zero the OFF_HANDLE value, keep the bucket occupied" trick
// ``ns_forget`` uses in ``tcl_ns.zig``.  Open-addressed probe chains
// stay intact; subsequent ``ns_cmd_find`` calls on the old name see
// the zero value and return "not found".
//
// Error semantics match ``tclsh 9.0`` message strings so
// diagnostic parity with upstream tests holds:
//
// * ``can't rename "X": command doesn't exist``  — missing source
// * ``can't rename to "Y": command already exists`` — target occupied
//
// Self-rename (``rename foo foo``) is a no-op, matching the C
// contract.  A hardcoded built-in protection list (``return``,
// ``error``) rejects renaming those with the ``ERR_BUILTIN`` code
// — the corresponding user-visible strings are formatted by the
// ``rename`` built-in wrapper in ``tcl_interp.zig`` so we don't
// pull the error surface into this module.

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;

const tcl_ns = @import("../interp/tcl_ns.zig");
const tcl_procs = @import("../interp/tcl_procs.zig");

/// Outcome of :func:`rename_command`.  Mapped onto the user-visible
/// error messages by the caller (``tcl_interp.zig`` ``rename`` built-in).
pub const RenameResult = enum(u32) {
    ok = 0,
    /// ``oldName`` didn't resolve to any Command.
    not_found = 1,
    /// Destination name was already occupied by a live Command.
    target_exists = 2,
    /// ``oldName`` names a hardcoded built-in we refuse to rename.
    builtin_protected = 3,
    /// Renaming this alias under the new name would form a
    /// dispatch cycle (interp-17.4 / 17.5 / 17.6).  Matches C
    /// Tcl's ``AliasCheckLoop`` returning ``TCL_ERROR``.
    alias_loop = 4,
};

/// Refuse to rename these hardcoded built-ins.  Matches the spirit of
/// Tcl 9's ``TclProtectedCommandsList`` without wiring a real trace
/// system.  Intentionally minimal — the list exists so tcltest's
/// accidental ``rename error`` shadow attempts surface as a clean
/// error instead of silently hollowing out the error mechanism.
const PROTECTED: []const []const u8 = &[_][]const u8{ "return", "error" };

fn is_protected(name_ptr: u32, name_len: u32) bool {
    if (name_len == 0) return false;
    const src: [*]const u8 = @ptrFromInt(name_ptr);
    inline for (PROTECTED) |p| {
        if (p.len == name_len) {
            var match = true;
            for (p, 0..) |c, i| {
                if (src[i] != c) {
                    match = false;
                    break;
                }
            }
            if (match) return true;
        }
    }
    return false;
}

/// Compare two byte spans for equality.  Small enough to inline.
fn bytes_eq(a_ptr: u32, a_len: u32, b_ptr: u32, b_len: u32) bool {
    if (a_len != b_len) return false;
    if (a_len == 0) return true;
    const ap: [*]const u8 = @ptrFromInt(a_ptr);
    const bp: [*]const u8 = @ptrFromInt(b_ptr);
    for (0..a_len) |i| {
        if (ap[i] != bp[i]) return false;
    }
    return true;
}

/// Write a fresh FQN onto ``cmd``'s name slot via the shared
/// :func:`tcl_ns.ns_build_fqn` formatter.  The previous buffer (if
/// any) is leaked via the bump allocator — command renames are
/// rare and bounded per program.  The stored name is the full
/// ``::path::to::simple`` form so ``proc_get_name_ptr`` consumers
/// (host dispatcher, ``info commands``) see the current identity.
fn write_command_fqn(cmd: u32, target_ns: u32, simple_ptr: u32, simple_len: u32) void {
    const fqn = tcl_ns.ns_build_fqn(target_ns, simple_ptr, simple_len);
    write_i32(cmd, @bitCast(fqn.ptr));
    write_i32(cmd + 4, @bitCast(fqn.len));
}

/// Walk every redirect that imported ``source_cmd`` and deactivate it
/// by zeroing its ``ImportedCmdData.real_cmd`` field.  After this the
/// redirects' probe-chain slots stay occupied (no tombstones) but
/// ``unwrap_imports`` sees ``real_cmd == 0`` and returns 0 —
/// dependent lookups start reporting "not found".
///
/// Mirrors the finalisation step in ``ns_forget`` but driven from the
/// source side: ``ns_forget`` walks redirects in a dest ns, this walks
/// redirects pointing at a source that's being removed.
/// Detect whether renaming an alias to ``new_name`` would form a
/// dispatch cycle.  Walks the alias's target chain using the same
/// algorithm as :fn:`alias_loop_would_form` in tcl_cmd_interp.zig
/// but starts from the alias's *current* target and treats
/// ``new_name`` (in the alias's child_interp) as the proposed
/// new identity.
fn alias_rename_would_loop(rec: *const @import("tcl_alias.zig").AliasRec, new_name_ptr: u32, new_name_len: u32) bool {
    const alias_mod = @import("tcl_alias.zig");
    const interp_reg = @import("../interp/tcl_interp_registry.zig");
    if (new_name_len == 0) return false;
    // The alias lives in ``child_interp``.  After rename its
    // identity becomes (child_interp, new_name); the target it
    // hops to is unchanged.  Walk the chain starting from
    // ``rec.target`` and see if we revisit (child_interp,
    // new_name).
    const new_simple = simple_name_of(new_name_ptr, new_name_len);
    const child_interp = rec.child_interp;
    if (child_interp == 0) return false;
    // Resolve the current target's effective interp: parent_interp
    // == 0 means same-as-child for cross-interp encoding.
    var cur_interp: u32 = if (rec.parent_interp == 0) child_interp else rec.parent_interp;
    var cur_name_ptr: u32 = rec.target_name_ptr;
    var cur_name_len: u32 = rec.target_name_len;
    const MAX_HOPS: u32 = 16;
    var hops: u32 = 0;
    while (hops < MAX_HOPS) : (hops += 1) {
        if (cur_interp == 0) return false;
        const cur_simple = simple_name_of(cur_name_ptr, cur_name_len);
        if (cur_interp == child_interp and cur_simple.len == new_simple.len) {
            const np: [*]const u8 = @ptrFromInt(new_simple.ptr);
            const cp: [*]const u8 = @ptrFromInt(cur_simple.ptr);
            var same = true;
            var k: u32 = 0;
            while (k < new_simple.len) : (k += 1) {
                if (np[k] != cp[k]) { same = false; break; }
            }
            if (same) return true;
        }
        // Walk one alias hop.
        const ci: *interp_reg.Interp = @ptrFromInt(cur_interp);
        const cmd2 = tcl_ns.ns_find_command(ci.root_ns, cur_simple.ptr, cur_simple.len);
        if (cmd2 == 0 or !alias_mod.is_alias(cmd2)) return false;
        const next_rec = alias_mod.alias_rec(cmd2);
        if (next_rec.target_name_len == 0) return false;
        cur_interp = if (next_rec.parent_interp == 0) cur_interp else next_rec.parent_interp;
        cur_name_ptr = next_rec.target_name_ptr;
        cur_name_len = next_rec.target_name_len;
    }
    return false;
}

fn simple_name_of(name_ptr: u32, name_len: u32) struct { ptr: u32, len: u32 } {
    if (name_len == 0) return .{ .ptr = name_ptr, .len = 0 };
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    var i: u32 = name_len;
    while (i >= 2) : (i -= 1) {
        if (sp[i - 2] == ':' and sp[i - 1] == ':') {
            return .{ .ptr = name_ptr + i, .len = name_len - i };
        }
    }
    return .{ .ptr = name_ptr, .len = name_len };
}

fn deactivate_importers(source_cmd: u32) void {
    var cur = read_i32(source_cmd + tcl_procs.OFF_IMPORT_REF_HEAD);
    while (cur != 0) {
        const node: *tcl_ns.ImportRef = @ptrFromInt(@as(u32, @bitCast(cur)));
        const redirect = node.imported_cmd;
        if (redirect != 0) {
            const desc = read_i32(redirect + tcl_procs.OFF_PARAMS_OBJ);
            if (desc != 0) {
                const d: *tcl_ns.ImportedCmdData = @ptrFromInt(@as(u32, @bitCast(desc)));
                d.real_cmd = 0;
            }
        }
        cur = @bitCast(node.next);
    }
    // Detach the list head — future importers of this name won't
    // accidentally see stale back-pointers.
    write_i32(source_cmd + tcl_procs.OFF_IMPORT_REF_HEAD, 0);
}

/// Internal: implement the full rename contract.  See module-level
/// docstring for the semantics table.  Does *not* format error
/// messages; returns a ``RenameResult`` the caller maps to Tcl's
/// user-visible strings.
///
/// When ``new_len == 0`` the Command is deleted (R5).  When both
/// ``old`` and ``new`` are non-empty, the Command moves.  The three
/// post-conditions below hold on success:
///
/// 1. The old name resolves to 0 (via ``ns_cmd_find``).
/// 2. The Command's on-disk FQN slot matches the new name.
/// 3. Every importer's ``unwrap_imports`` result is unchanged (still
///    points at the same Command pointer, which has now moved to the
///    new name).  Because ``ImportedCmdData.real_cmd`` stores a
///    pointer and the Command's linear-memory address doesn't change
///    on rename, no cascade is needed.  Callers that cache the
///    Command by name (the proc-lookup LRU) are flushed separately
///    via ``tcl_procs.lru_invalidate_all``.
pub fn rename_command(
    old_ns: u32,
    old_simple_ptr: u32,
    old_simple_len: u32,
    new_ns: u32,
    new_simple_ptr: u32,
    new_simple_len: u32,
) RenameResult {
    if (is_protected(old_simple_ptr, old_simple_len)) return .builtin_protected;

    const cmd = tcl_ns.ns_cmd_find(old_ns, old_simple_ptr, old_simple_len);
    if (cmd == 0) return .not_found;

    // Self-rename: no-op.  Matches C's check after
    // ``TclGetCommandFromObj`` when ``oldFullName == newFullName``.
    if (old_ns == new_ns and bytes_eq(
        old_simple_ptr,
        old_simple_len,
        new_simple_ptr,
        new_simple_len,
    )) return .ok;

    // Alias-rename loop check: when renaming an alias, the new
    // name + the alias's target must not form a cycle once the
    // rename is applied.  Mirrors ``AliasCheckLoop`` in
    // tclInterp.c which runs on both create and rename paths
    // (interp-17.4 / 17.5 / 17.6).
    if (new_simple_len > 0) {
        const alias_mod = @import("tcl_alias.zig");
        if (alias_mod.is_alias(cmd)) {
            const rec = alias_mod.alias_rec(cmd);
            if (alias_rename_would_loop(rec, new_simple_ptr, new_simple_len)) {
                return .alias_loop;
            }
        }
    }

    // Delete form: new_simple is empty.  Clear the source bucket,
    // deactivate every redirect that imports it.  We don't splice
    // this command out of any lists it might be on itself (it's the
    // source side of the relationship, not a redirect) — the
    // ``import_ref_head`` is the one we walk.
    if (new_simple_len == 0) {
        // If the command being deleted is a CMD_IMPORTED redirect,
        // splice it out of the source's back-list so a later delete
        // of the source doesn't try to touch the (now-gone) redirect.
        const flags: u32 = @bitCast(read_i32(cmd + tcl_procs.OFF_FLAGS));
        if ((flags & tcl_procs.CMD_IMPORTED) != 0) {
            const desc = read_i32(cmd + tcl_procs.OFF_PARAMS_OBJ);
            if (desc != 0) {
                const d: *tcl_ns.ImportedCmdData = @ptrFromInt(@as(u32, @bitCast(desc)));
                if (d.real_cmd != 0) tcl_ns.unlink_import_ref(d.real_cmd, cmd);
                d.real_cmd = 0;
            }
        } else {
            // Source side: deactivate every redirect pointing at us.
            deactivate_importers(cmd);
        }
        // If the command is an alias, unhook the AliasRec from
        // the per-interp chains so ``interp aliases`` no longer
        // reports the deleted entry (interp-19.7 / 19.8).  Match
        // C Tcl's ``TclDeleteAlias`` which removes the
        // ``aliasTable`` entry whenever the source command is
        // destroyed for any reason.
        const alias_mod = @import("tcl_alias.zig");
        if (alias_mod.is_alias(cmd)) {
            alias_mod.alias_clear(cmd);
        }
        _ = tcl_ns.ns_cmd_clear(old_ns, old_simple_ptr, old_simple_len);
        tcl_procs.lru_invalidate_all();
        return .ok;
    }

    // Move form: the destination name must be unoccupied in new_ns.
    // Zero-valued (tombstoned) buckets count as vacant — ``ns_cmd_find``
    // returns 0 for them, so this check naturally allows re-use of
    // a previously-deleted slot.
    const target_occupant = tcl_ns.ns_cmd_find(new_ns, new_simple_ptr, new_simple_len);
    if (target_occupant != 0) return .target_exists;

    // Perform the move.  Order matters: insert at new first (so the
    // Command has a live home before the old entry is cleared — this
    // keeps any concurrent re-entrant lookup finding *something*).
    _ = tcl_ns.ns_cmd_put(new_ns, new_simple_ptr, new_simple_len, cmd);
    _ = tcl_ns.ns_cmd_clear(old_ns, old_simple_ptr, old_simple_len);

    // Update the Command's own FQN slot so ``proc_get_name_ptr``,
    // ``info commands``, and the host-bridge dispatcher see the new
    // identity.  CMD_IMPORTED redirects keep their real_cmd pointer
    // intact — only the Command's name slot moves, which matches the
    // C contract where renaming a redirect doesn't touch the source.
    //
    // Compiled procs used to be an exception here — their dispatch
    // key was the Command's live name slot, so rewriting it would
    // break host-bridge lookup.  The sidecar at
    // ``OFF_EXPORT_NAME_BUCKET`` (populated by
    // ``proc_register_compiled``) now carries the immutable
    // registration-time export name, and ``tcl_dispatch.dispatch``
    // reads it first — so renaming a compiled proc is safe and the
    // live name slot can always reflect the user-visible identity.
    write_command_fqn(cmd, new_ns, new_simple_ptr, new_simple_len);

    // ``ns_cmd_put`` and ``ns_cmd_clear`` each bump their respective
    // ns's ``cmd_ref_epoch`` and cascade through ``path_source_head``,
    // so path-based invalidation is already covered.  The proc-lookup
    // LRU is an orthogonal cache keyed on ``(ns, hash, name)`` —
    // nuke it so no stale entry keeps resolving the old name.
    tcl_procs.lru_invalidate_all();
    return .ok;
}
