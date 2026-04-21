// Interpreter-wide ``interp hide`` / ``interp expose`` support.
//
// Mirror of Tcl 9's ``Tcl_HideCommand`` / ``Tcl_ExposeCommand`` in
// ``tclBasic.c`` (and the ``HiddenObjCmd`` / ``ExposeObjCmd`` wrappers
// in ``tclInterp.c``), trimmed to the single-interp subset that
// matches the rest of the rename / alias wave.  Child-interpreter
// cross-interp hide/expose remain deferred.
//
// Semantics summary (see ``docs/design/runtime/command-introspection.md``
// §2 for the full picture):
//
//   interp hide {} cmd ?hiddenName?
//     - Resolves ``cmd`` to a live Command in the current namespace
//       tree.  Rejects qualified source names (``interp hide :: ::foo::bar``
//       is an error — hidden commands always live in the interpreter-
//       wide hidden table, not in any namespace).
//     - Removes the Command from its home ns's ``cmd_table`` via
//       ``ns_cmd_clear`` (tombstone-the-bucket; same technique
//       ``rename`` uses for the delete form).
//     - Inserts the Command into the interpreter-wide
//       ``hidden_cmd_table`` under ``hiddenName`` (defaults to
//       ``cmd``'s simple name).  Rejects collisions with an
//       existing hidden entry.
//     - Walks the Command's ``import_ref_head`` list and
//       deactivates every redirect — same cascade rename-to-empty
//       performs, so importers see "command not found" on next
//       dispatch.
//
//   interp expose {} hiddenName ?newName?
//     - Resolves ``hiddenName`` to a Command in the hidden table.
//     - Rejects qualified ``newName`` — the destination must always
//       be a simple name, placed into the *current* namespace.
//     - Refuses to overwrite a live Command at the destination.
//     - Moves the Command into ``current_ns``'s ``cmd_table``,
//       clears the hidden-table slot, rewrites the Command's stored
//       FQN to match the new location.
//
// Both sides leave the Command's ``import_ref_head`` at zero after
// the move: live importers have been deactivated on hide, and
// expose doesn't reattach them (matches C Tcl, where re-exposing a
// previously-imported source requires a fresh ``namespace import``).

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const tcl_ns = @import("tcl_ns.zig");
const tcl_procs = @import("tcl_procs.zig");
const interp_reg = @import("tcl_interp_registry.zig");

/// Outcome of :func:`hide_command`.  Mapped onto user-visible error
/// messages by the ``interp hide`` built-in in ``tcl_interp.zig``.
pub const HideResult = enum(u32) {
    ok = 0,
    /// ``cmd`` didn't resolve to any live Command.
    not_found = 1,
    /// Caller passed a qualified source name
    /// (``::foo::bar``) — hidden commands must be identified by their
    /// simple name in the current namespace.
    qualified_name_rejected = 2,
    /// An entry is already live in the hidden table under
    /// ``hiddenName``.
    hidden_name_taken = 3,
};

/// Outcome of :func:`expose_command`.
pub const ExposeResult = enum(u32) {
    ok = 0,
    /// ``hiddenName`` didn't match any hidden-table entry.
    not_found = 1,
    /// Caller passed a qualified ``newName`` — the destination must
    /// always be a simple name in the current namespace.
    qualified_name_rejected = 2,
    /// A live Command is already registered under the destination
    /// name in the current namespace.
    target_exists = 3,
};

/// Scan a byte range for any ``::`` substring.  Used to enforce the
/// "simple name only" rule on both source and destination names.
fn has_qualifier(ptr: u32, len: u32) bool {
    if (len < 2) return false;
    const src: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    while (i + 1 < len) : (i += 1) {
        if (src[i] == ':' and src[i + 1] == ':') return true;
    }
    return false;
}

/// Deactivate every import redirect pointing at ``source_cmd`` by
/// zeroing its ``ImportedCmdData.real_cmd``.  Mirrors the source-side
/// cascade in ``tcl_rename`` without a circular import: we walk the
/// singly-linked ``ImportRef`` list rooted at ``OFF_IMPORT_REF_HEAD``
/// and clear each redirect's ``real_cmd`` pointer.
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
    write_i32(source_cmd + tcl_procs.OFF_IMPORT_REF_HEAD, 0);
}

/// Implement ``interp hide ?path? cmd ?hiddenName?``.  ``src_*`` is
/// the simple source name (qualified names already rejected by the
/// caller via :func:`has_qualifier`); ``hidden_*`` is the target
/// name in the hidden table (defaults to ``src_*`` when caller
/// didn't provide an explicit destination).  ``src_ns`` is the
/// namespace where the source currently lives — typically the target
/// interp's ``root_ns`` at the call site.  ``target_interp`` is the
/// interp whose hidden table receives the Command; the single-interp
/// form passes ``interp_reg.interp_current()``.
pub fn hide_command(
    target_interp: u32,
    src_ns: u32,
    src_simple_ptr: u32,
    src_simple_len: u32,
    hidden_name_ptr: u32,
    hidden_name_len: u32,
) HideResult {
    if (has_qualifier(src_simple_ptr, src_simple_len)) return .qualified_name_rejected;
    if (has_qualifier(hidden_name_ptr, hidden_name_len)) return .qualified_name_rejected;

    const cmd = tcl_ns.ns_cmd_find(src_ns, src_simple_ptr, src_simple_len);
    if (cmd == 0) return .not_found;

    // Refuse to overwrite an existing hidden entry.  Matches C Tcl's
    // ``Tcl_HideCommand`` which raises
    // ``hidden command named "X" already exists``.
    if (interp_reg.hidden_find(target_interp, hidden_name_ptr, hidden_name_len) != 0) {
        return .hidden_name_taken;
    }

    // Deactivate any redirects pointing at this Command before we
    // tombstone the source bucket.  Hidden commands are invisible
    // to ``proc_lookup`` so importers would otherwise keep
    // resolving through a now-unreachable source.
    deactivate_importers(cmd);

    // Move: clear source bucket, stash Command in the hidden table
    // under ``hidden_name``, rewrite the Command's stored name slot
    // so introspection returns the hidden identity (matches C Tcl's
    // behaviour — ``Tcl_GetCommandName`` on a hidden command
    // returns the hidden name).  We *don't* rebuild an FQN because
    // hidden commands have no namespace parent; the hidden name
    // stands alone.
    _ = interp_reg.hidden_put(target_interp, hidden_name_ptr, hidden_name_len, cmd);
    _ = tcl_ns.ns_cmd_clear(src_ns, src_simple_ptr, src_simple_len);

    const nbuf = alloc(hidden_name_len);
    if (hidden_name_len > 0) memcpy(nbuf, hidden_name_ptr, hidden_name_len);
    write_i32(cmd, @bitCast(nbuf));
    write_i32(cmd + 4, @bitCast(hidden_name_len));

    tcl_procs.lru_invalidate_all();
    return .ok;
}

/// Implement ``interp expose ?path? hiddenName ?newName?``.  ``new_*``
/// is the simple destination name in ``dest_ns`` (defaults to
/// ``hidden_*`` when caller didn't provide an explicit destination).
/// ``target_interp`` is the interp whose hidden table the entry
/// currently lives in; single-interp callers pass
/// ``interp_reg.interp_current()``.
pub fn expose_command(
    target_interp: u32,
    hidden_name_ptr: u32,
    hidden_name_len: u32,
    dest_ns: u32,
    new_simple_ptr: u32,
    new_simple_len: u32,
) ExposeResult {
    if (has_qualifier(new_simple_ptr, new_simple_len)) return .qualified_name_rejected;

    const cmd = interp_reg.hidden_find(target_interp, hidden_name_ptr, hidden_name_len);
    if (cmd == 0) return .not_found;

    if (tcl_ns.ns_cmd_find(dest_ns, new_simple_ptr, new_simple_len) != 0) {
        return .target_exists;
    }

    // Move: insert first so the Command has a live home throughout
    // the transition (see ``rename_command`` for the same ordering
    // rationale), then clear the hidden-table slot.  The Command's
    // stored name slot gets rewritten to its new fully-qualified
    // form so ``info commands`` and the host-bridge dispatcher see
    // the exposed identity.
    _ = tcl_ns.ns_cmd_put(dest_ns, new_simple_ptr, new_simple_len, cmd);
    _ = interp_reg.hidden_clear(target_interp, hidden_name_ptr, hidden_name_len);

    const fqn = tcl_ns.ns_build_fqn(dest_ns, new_simple_ptr, new_simple_len);
    write_i32(cmd, @bitCast(fqn.ptr));
    write_i32(cmd + 4, @bitCast(fqn.len));

    tcl_procs.lru_invalidate_all();
    return .ok;
}
