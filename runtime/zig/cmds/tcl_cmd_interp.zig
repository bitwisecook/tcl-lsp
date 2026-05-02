// Tcl command management: interp, rename, namespace-which.
//
// Implements the ``interp``, ``rename``, and ``namespace which``
// command groups.  These do not call eval_script or eval_command,
// so they can be imported from tcl_interp.zig without a cycle.
// Renamed from tcl_interp_interp.zig (the old name was misleading).

const rt = @import("../tcl_runtime.zig");
const procs = @import("../interp/tcl_procs.zig");
const obj_mod = @import("../valtypes/tcl_obj.zig");

const alloc = rt.alloc;
const memcpy = rt.memcpy;
const read_i32 = obj_mod.read_i32;
const write_i32 = obj_mod.write_i32;
const obj_new_string = rt.obj_new_string;
const obj_new_int = rt.obj_new_int;
const obj_new_string_copy = rt.obj_new_string_copy;
const obj_ensure_string = rt.obj_ensure_string;
const list_count_elements = rt.list_count_elements;
const list_element_at = rt.list_element_at;

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;
const tcl_ns = @import("../interp/tcl_ns.zig");
const alias_mod = @import("tcl_alias.zig");
const rename_mod = @import("tcl_rename.zig");
const hide_mod = @import("tcl_hide.zig");
const interp_reg = @import("../interp/tcl_interp_registry.zig");

// -- ``rename`` built-in -------------------------------------------------------
//
// ``rename oldName newName``.  ``newName == ""`` deletes ``oldName``.
// Semantics live in ``tcl_rename.zig``; this wrapper parses argv,
// resolves the ``(old_ns, old_simple)`` / ``(new_ns, new_simple)``
// pairs via the qualified-name walker, and formats the user-visible
// error messages.


/// Build an error message like ``can't rename "foo": command doesn't
/// exist`` and route it through the standard error trap.  The
/// per-case templates come from ``tclsh 9.0`` verbatim so tcltest's
/// error-string matchers behave identically.
pub fn rename_error(template_prefix: []const u8, name_ptr: u32, name_len: u32, template_suffix: []const u8) void {
    const total: u32 = @intCast(template_prefix.len + name_len + template_suffix.len);
    const buf = alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (template_prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (name_len > 0) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| dst[off + k] = src[k];
        off += name_len;
    }
    for (template_suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const msg = obj_new_string(@bitCast(buf), @bitCast(off));
    const catch_mod = @import("../interp/tcl_catch.zig");
    catch_mod.tcl_cmd_error(msg);
}

/// Implement ``rename BUILTIN newName`` (and ``rename BUILTIN ""``).
/// Synthesises the ``CMD_BUILTIN_FORWARD`` for the new name (if any)
/// and the ``CMD_BUILTIN_MASKED`` tombstone for the old name so the
/// proc-lookup fast path in ``eval_command`` sees them before the
/// BUILTIN cmd_table.  The BUILTIN handler itself is the data
/// stashed in the FORWARD's ``params_obj`` slot.
fn rename_builtin(
    old_s: anytype,
    new_s: anytype,
    handler: @import("../dispatch/tcl_cmd_registry.zig").HandlerFn,
) i32 {
    // Refuse to rename the protected BUILTINs (``return``, ``error``).
    // The rename module's ``is_protected`` check fires for those when
    // they reach the proc-table path, but we route around it for
    // BUILTIN renames so the protection lives here.
    const protected: []const []const u8 = &[_][]const u8{ "return", "error" };
    inline for (protected) |p| {
        if (p.len == old_s.len) {
            const op: [*]const u8 = @ptrFromInt(old_s.ptr);
            var match = true;
            for (p, 0..) |c, i| {
                if (op[i] != c) {
                    match = false;
                    break;
                }
            }
            if (match) {
                rename_error("can't rename \"", old_s.ptr, old_s.len, "\": built-in command");
                return 0;
            }
        }
    }

    // Move form: refuse if the destination name is occupied by a
    // live (non-tombstone) command.  Matches reference Tcl's
    // ``can't rename to "X": command already exists``.
    if (new_s.len > 0) {
        const cxt = tcl_ns.ns_current();
        const new_r = tcl_ns.ns_resolve_qualified(cxt, new_s.ptr, new_s.len);
        const new_target_ns = if (new_r.target_ns != 0) new_r.target_ns else new_r.alt_ns;
        if (new_target_ns != 0) {
            const occ = tcl_ns.ns_cmd_find(new_target_ns, new_r.simple_ptr, new_r.simple_len);
            if (occ != 0) {
                const f: u32 = @bitCast(read_i32(occ + procs.OFF_FLAGS));
                if ((f & procs.CMD_BUILTIN_MASKED) == 0) {
                    rename_error("can't rename to \"", new_s.ptr, new_s.len, "\": command already exists");
                    return 0;
                }
            }
        }
        // The proc-table check above only sees user-defined / proc-
        // registered Commands.  Hardcoded BUILTINs (``set``,
        // ``list``, ``puts``, …) live in the static dispatch table
        // and have no proc record until ``rename`` masks/forwards
        // them.  ``rename list set`` would otherwise install a
        // forwarder over ``set`` and silently shadow the builtin —
        // proc dispatch runs before the BUILTIN cmd_table, so the
        // forwarder would win and ``set`` would lose its identity.
        // Match reference Tcl by raising ``can't rename to
        // "set": command already exists`` (Codex review on
        // PR #325).
        const cmd_table = @import("../dispatch/tcl_cmd_table.zig");
        if (cmd_table.lookup(new_r.simple_ptr, new_r.simple_len) != null) {
            rename_error("can't rename to \"", new_s.ptr, new_s.len, "\": command already exists");
            return 0;
        }
        const handler_addr: u32 = @intCast(@intFromPtr(handler));
        procs.register_builtin_forward(new_s.ptr, new_s.len, handler_addr);
    }

    // Mask the BUILTIN under its old name so subsequent dispatch
    // surfaces ``invalid command name`` instead of running it.
    procs.register_builtin_masked(old_s.ptr, old_s.len);
    return 0;
}

pub fn eval_rename(words: []const i32) i32 {
    // ``rename`` takes exactly two operands (oldName + newName).
    // Extra words are rejected with ``wrong # args`` rather than
    // silently ignored — matches Tcl 9's ``RenameObjCmd``.
    if (words.len != 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        // Literal's ``.len`` is authoritative — hard-coding drifts.
        const err_text = "wrong # args: should be \"rename oldName newName\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const old_s = obj_ensure_string(words[1]);
    const new_s = obj_ensure_string(words[2]);

    const cxt = tcl_ns.ns_current();
    const old_r = tcl_ns.ns_resolve_qualified(cxt, old_s.ptr, old_s.len);
    // Resolve old name: prefer the primary target, fall back to alt
    // (search-from-root when the primary context misses).
    var old_ns: u32 = 0;
    if (old_r.target_ns != 0 and
        tcl_ns.ns_cmd_find(old_r.target_ns, old_r.simple_ptr, old_r.simple_len) != 0)
    {
        old_ns = old_r.target_ns;
    } else if (old_r.alt_ns != 0 and
        tcl_ns.ns_cmd_find(old_r.alt_ns, old_r.simple_ptr, old_r.simple_len) != 0)
    {
        old_ns = old_r.alt_ns;
    } else {
        // No live Command in the proc table for this name — but it
        // may still be a hardcoded BUILTIN.  Route to the
        // BUILTIN-rename path which synthesises a CMD_BUILTIN_FORWARD
        // (for the new name) and a CMD_BUILTIN_MASKED tombstone (for
        // the old name) so subsequent dispatch sees them via the
        // proc-lookup fast path.  See rename.test rename-2.1.
        const cmd_table = @import("../dispatch/tcl_cmd_table.zig");
        if (cmd_table.lookup(old_s.ptr, old_s.len)) |handler| {
            return rename_builtin(old_s, new_s, handler);
        }
        // ``::name``-qualified BUILTIN — strip and retry.
        if (old_s.len >= 2) {
            const old_p: [*]const u8 = @ptrFromInt(old_s.ptr);
            if (old_p[0] == ':' and old_p[1] == ':') {
                if (cmd_table.lookup(old_s.ptr + 2, old_s.len - 2)) |handler| {
                    return rename_builtin(old_s, new_s, handler);
                }
            }
        }
        // Truly unknown — emit ``can't rename`` (move) or
        // ``can't delete`` (delete) per reference Tcl.
        if (new_s.len == 0) {
            rename_error("can't delete \"", old_s.ptr, old_s.len, "\": command doesn't exist");
        } else {
            rename_error("can't rename \"", old_s.ptr, old_s.len, "\": command doesn't exist");
        }
        return 0;
    }

    // Found a live Command.  Special-case ``rename FORWARD MASKED-name``
    // (the inverse of an earlier BUILTIN rename) so the BUILTIN slot
    // becomes dispatchable again — delete both the FORWARD and the
    // tombstone, leaving the BUILTIN cmd_table to win on next lookup.
    const old_cmd = tcl_ns.ns_cmd_find(old_ns, old_r.simple_ptr, old_r.simple_len);
    if (old_cmd != 0) {
        const old_flags: u32 = @bitCast(read_i32(old_cmd + procs.OFF_FLAGS));
        if ((old_flags & procs.CMD_BUILTIN_FORWARD) != 0 and new_s.len > 0) {
            const cmd_table = @import("../dispatch/tcl_cmd_table.zig");
            // Look up the BUILTIN that corresponds to the destination
            // name.  When the dest is a name shadowed by a MASKED
            // tombstone for this same BUILTIN, the rename undoes the
            // earlier BUILTIN rename.
            const new_r_probe = tcl_ns.ns_resolve_qualified(cxt, new_s.ptr, new_s.len);
            const new_target_ns = if (new_r_probe.target_ns != 0)
                new_r_probe.target_ns
            else
                new_r_probe.alt_ns;
            const new_cmd = if (new_target_ns != 0)
                tcl_ns.ns_cmd_find(new_target_ns, new_r_probe.simple_ptr, new_r_probe.simple_len)
            else
                0;
            const new_is_masked = if (new_cmd != 0) blk: {
                const f: u32 = @bitCast(read_i32(new_cmd + procs.OFF_FLAGS));
                break :blk (f & procs.CMD_BUILTIN_MASKED) != 0;
            } else false;
            if (new_is_masked and cmd_table.lookup(new_s.ptr, new_s.len) != null) {
                _ = procs.unregister_command(old_s.ptr, old_s.len);
                _ = procs.unregister_command(new_s.ptr, new_s.len);
                return 0;
            }
        }
    }

    // Deletion form: new name is empty.
    if (new_s.len == 0) {
        const r = rename_mod.rename_command(
            old_ns,
            old_r.simple_ptr,
            old_r.simple_len,
            0,
            0,
            0,
        );
        switch (r) {
            .ok => return 0,
            .not_found => {
                rename_error("can't rename \"", old_s.ptr, old_s.len, "\": command doesn't exist");
                return 0;
            },
            .builtin_protected => {
                rename_error("can't rename \"", old_s.ptr, old_s.len, "\": built-in command");
                return 0;
            },
            .target_exists => return 0, // unreachable on delete form
        }
    }

    // Move form: resolve new name, materialising missing intermediate
    // namespaces the way ``proc`` does for its registration path.
    const new_r = tcl_ns.ns_resolve_qualified_creating(cxt, new_s.ptr, new_s.len);
    if (new_r.target_ns == 0 or new_r.simple_len == 0) {
        rename_error("can't rename to \"", new_s.ptr, new_s.len, "\": invalid name");
        return 0;
    }
    const r = rename_mod.rename_command(
        old_ns,
        old_r.simple_ptr,
        old_r.simple_len,
        new_r.target_ns,
        new_r.simple_ptr,
        new_r.simple_len,
    );
    switch (r) {
        .ok => return 0,
        .not_found => {
            rename_error("can't rename \"", old_s.ptr, old_s.len, "\": command doesn't exist");
            return 0;
        },
        .target_exists => {
            rename_error("can't rename to \"", new_s.ptr, new_s.len, "\": command already exists");
            return 0;
        },
        .builtin_protected => {
            rename_error("can't rename \"", old_s.ptr, old_s.len, "\": built-in command");
            return 0;
        },
    }
}
// -- ``interp`` built-in -------------------------------------------------------
//
// Currently only ``interp alias`` is implemented, in its
// single-interp form: ``interp alias {} newName {} target ?arg …?``
// creates / queries / deletes an alias in the (only) interpreter.
// Child-interp aliases and the other ``interp`` sub-commands (eval,
// hide, expose, create, delete, …) remain trapping stubs via
// :mod:`tcl_env_stubs`.

/// Full set of ``interp`` subcommand names, ordered to match tclsh's
/// ``bad option`` error messages.  Unrecognised subcommands emit
/// a ``bad option "X": must be alias, aliases, ...`` error with
/// this exact ordering (verbatim from
/// ``tmp/tcl9.0.3/generic/tclInterp.c``'s ``interpOptions`` table).
const INTERP_SUBCOMMAND_LIST: []const u8 = "alias, aliases, bgerror, cancel, children, create, debug, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, limit, marktrusted, recursionlimit, share, target, or transfer";

pub fn emit_bad_option(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "bad option \"";
    const infix: []const u8 = "\": must be ";
    const total: u32 = @as(u32, @intCast(prefix.len)) +
        name_len +
        @as(u32, @intCast(infix.len)) +
        @as(u32, @intCast(INTERP_SUBCOMMAND_LIST.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |b, k| d[k] = b;
    if (name_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| d[prefix.len + k] = sp[k];
    }
    for (infix, 0..) |b, k| d[prefix.len + name_len + k] = b;
    for (INTERP_SUBCOMMAND_LIST, 0..) |b, k| {
        d[prefix.len + name_len + infix.len + k] = b;
    }
    const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}
pub fn interp_alias_create(
    child_interp: u32,
    parent_interp: u32,
    new_name_ptr: u32,
    new_name_len: u32,
    target_name_ptr: u32,
    target_name_len: u32,
    n_prefix: u32,
    prefix_buf: u32,
) i32 {
    // The alias lives in ``child_interp`` (its source side).  The
    // target is resolved at dispatch time against ``parent_interp``.
    // Resolution context is the child interp's root ns when the
    // caller and child differ; otherwise the caller's current ns (to
    // preserve single-interp ``namespace eval`` placement).
    const child: *interp_reg.Interp = @ptrFromInt(child_interp);
    const cxt: u32 = if (child_interp == interp_reg.interp_current())
        tcl_ns.ns_current()
    else
        child.root_ns;

    // Swap into the child interp for the resolve-creating call so
    // intermediate namespaces land in the child's tree when the
    // alias name carries qualifiers (``interp alias {} ::ns::foo
    // ...`` stays in the parent, but ``interp alias child ::ns::foo
    // ...`` must create ``::ns`` inside the child).
    const swapped_child = child_interp != interp_reg.interp_current();
    const save = if (swapped_child) interp_reg.enter(child_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    const r = tcl_ns.ns_resolve_qualified_creating(cxt, new_name_ptr, new_name_len);
    if (r.target_ns == 0 or r.simple_len == 0) {
        interp_reg.leave(save);
        return 0;
    }

    // If an alias / command already lives under this name, replace
    // it.  This matches C Tcl where ``interp alias {} foo {} bar``
    // overwrites any previous ``foo`` (proc, alias, or otherwise).
    // The previous Command stays in linear memory — leaked per the
    // bump-allocator contract.
    //
    // The parent interp is stashed directly on the ``AliasRec``
    // (via ``alias_alloc``'s last parameter) rather than on the
    // Command's ``OFF_IMPORT_REF_HEAD`` slot: that slot is shared
    // with the ``namespace import`` back-reference list head, so a
    // later ``namespace import`` of this alias could overwrite the
    // stashed Interp* and corrupt cross-interp dispatch.  Zero
    // here means "same-interp dispatch; no swap needed".
    const stash_parent: u32 = if (parent_interp != child_interp) parent_interp else 0;
    const cmd = alias_mod.alias_alloc(
        r.target_ns,
        r.simple_ptr,
        r.simple_len,
        target_name_ptr,
        target_name_len,
        n_prefix,
        prefix_buf,
        stash_parent,
    );
    _ = tcl_ns.ns_cmd_put(r.target_ns, r.simple_ptr, r.simple_len, cmd);
    interp_reg.leave(save);
    // Bump the proc counter so ``proc_buf_nonzero`` fires for
    // bundles whose only commands are aliases.
    procs.proc_count_bump();
    return words_obj_new_string_dup(new_name_ptr, new_name_len);
}

pub fn interp_alias_query(child_interp: u32, new_name_ptr: u32, new_name_len: u32) i32 {
    const child: *interp_reg.Interp = @ptrFromInt(child_interp);
    const cxt: u32 = if (child_interp == interp_reg.interp_current())
        tcl_ns.ns_current()
    else
        child.root_ns;
    const swapped_child = child_interp != interp_reg.interp_current();
    const save = if (swapped_child) interp_reg.enter(child_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    const cmd = tcl_ns.ns_find_command(cxt, new_name_ptr, new_name_len);
    interp_reg.leave(save);
    if (!alias_mod.is_alias(cmd)) return 0;
    return alias_mod.alias_describe(cmd);
}

pub fn interp_alias_delete(child_interp: u32, new_name_ptr: u32, new_name_len: u32) i32 {
    const child: *interp_reg.Interp = @ptrFromInt(child_interp);
    const cxt: u32 = if (child_interp == interp_reg.interp_current())
        tcl_ns.ns_current()
    else
        child.root_ns;
    const swapped_child = child_interp != interp_reg.interp_current();
    const save = if (swapped_child) interp_reg.enter(child_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    const r = tcl_ns.ns_resolve_qualified(cxt, new_name_ptr, new_name_len);
    const host_ns: u32 = if (r.target_ns != 0 and
        tcl_ns.ns_cmd_find(r.target_ns, r.simple_ptr, r.simple_len) != 0)
        r.target_ns
    else if (r.alt_ns != 0)
        r.alt_ns
    else {
        interp_reg.leave(save);
        return 0;
    };
    const cmd = tcl_ns.ns_cmd_find(host_ns, r.simple_ptr, r.simple_len);
    if (alias_mod.is_alias(cmd)) {
        alias_mod.alias_clear(cmd);
    }
    _ = tcl_ns.ns_cmd_clear(host_ns, r.simple_ptr, r.simple_len);
    interp_reg.leave(save);
    procs.lru_invalidate_all();
    return 0;
}

/// Return a TclObj wrapping a fresh string copy of the given bytes.
/// Tiny helper to avoid pulling in ``obj_new_string_copy``'s ABI
/// naming at every callsite.
pub fn words_obj_new_string_dup(ptr: u32, len: u32) i32 {
    const buf = alloc(len);
    if (len > 0) memcpy(buf, ptr, len);
    return obj_new_string(@bitCast(buf), @bitCast(len));
}

/// ``interp aliases ?path?`` — list every registered alias in the
/// resolved interp.  Walks the interp's namespace tree starting at
/// its root, visiting each ``cmd_table`` once and emitting commands
/// flagged ``CMD_ALIAS``.  Output is a Tcl list of simple alias
/// names (not FQNs) — matches ``tclsh``'s default.
pub fn interp_aliases_list() i32 {
    return interp_aliases_list_for(interp_reg.interp_current());
}

/// Shared implementation: list aliases reachable from ``target_interp``.
/// Child-as-command (`<child> aliases`) and the explicit
/// ``interp aliases <path>`` form both route here.
pub fn interp_aliases_list_for(target_interp: u32) i32 {
    if (target_interp == 0) return obj_new_string(0, 0);
    const t: *interp_reg.Interp = @ptrFromInt(target_interp);
    // Accumulator: sum string lengths (plus separators) to size the
    // output buffer.  We walk the tree twice: once to size, once to
    // fill.  Single-pass grown allocation would require a realloc
    // path the bump allocator doesn't support.  Both passes route
    // through the shared ``tcl_ns.walk_tree_cmd_tables`` helper so
    // every ns-tree introspection walker shares one recursion shape.
    var ctx: AliasListCtx = .{ .total = 0, .count = 0, .buf = 0, .off = 0 };
    tcl_ns.walk_tree_cmd_tables(t.root_ns, &ctx, alias_size_visit);
    if (ctx.total == 0) return obj_new_string(0, 0);

    ctx.buf = alloc(ctx.total);
    ctx.off = 0;
    ctx.count = 0;
    tcl_ns.walk_tree_cmd_tables(t.root_ns, &ctx, alias_fill_visit);
    return obj_new_string(@bitCast(ctx.buf), @bitCast(ctx.off));
}

const AliasListCtx = struct {
    total: u32,
    count: u32,
    buf: u32,
    off: u32,
};

/// Visitor for the sizing pass of ``interp aliases``.  Inspects
/// each bucket, filters to aliases, and accumulates the worst-case
/// list-quoted length plus a separator byte into ``ctx.total``.  C
/// Tcl's ``interp aliases`` emits each alias's simple name only —
/// ``{bar foo}``, not ``{::bar ::foo}`` — list-quoted so names
/// containing whitespace / braces / backslashes / leading ``#``
/// round-trip correctly via ``lindex`` / ``foreach``.
pub fn alias_size_visit(ctx: *AliasListCtx, _: u32, _: u32, name_len: u32, cmd: u32) void {
    if (!alias_mod.is_alias(cmd)) return;
    if (ctx.count > 0) ctx.total += 1;
    // Worst-case from ``list_elem_quote``: ``2 * name_len + 2``.
    // Empty names expand to ``{}`` (2 bytes).
    ctx.total += if (name_len == 0) 2 else (2 * name_len + 2);
    ctx.count += 1;
}

/// Visitor for the filling pass of ``interp aliases``.  Writes the
/// alias's simple name into ``ctx.buf`` at ``ctx.off`` via
/// :func:`tcl_list_quote.list_elem_quote` (element 0) or
/// :func:`list_elem_quote_nth` (subsequent), space-separated from
/// prior entries.
pub fn alias_fill_visit(ctx: *AliasListCtx, _: u32, name_ptr: u32, name_len: u32, cmd: u32) void {
    if (!alias_mod.is_alias(cmd)) return;
    const list_quote = @import("../valtypes/tcl_list_quote.zig");
    if (ctx.count > 0) {
        const d: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
        d[0] = ' ';
        ctx.off += 1;
    }
    ctx.off = if (ctx.count == 0)
        list_quote.list_elem_quote(ctx.buf, ctx.off, name_ptr, name_len)
    else
        list_quote.list_elem_quote_nth(ctx.buf, ctx.off, name_ptr, name_len);
    ctx.count += 1;
}

// -- ``interp hide`` / ``interp expose`` / ``interp hidden`` ---------------


pub fn interp_hide_error(prefix: []const u8, name_ptr: u32, name_len: u32, suffix: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |b, k| d[k] = b;
    if (name_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| d[prefix.len + k] = sp[k];
    }
    for (suffix, 0..) |b, k| d[prefix.len + name_len + k] = b;
    const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}

/// Resolve an ``interp`` subcommand path argument to an ``Interp*``.
/// Matches ``GetInterp`` in ``tclInterp.c``: empty path means "this
/// interp"; a non-empty Tcl list is walked down the child chain
/// from the current interp.  On miss raises the standard tclsh
/// error ``could not find interpreter "X"`` and returns 0.
pub fn resolve_interp_path(path_obj: i32) u32 {
    const s = obj_ensure_string(path_obj);
    const base = interp_reg.interp_current();
    if (s.len == 0) return base;
    const target = interp_reg.resolve_path(base, s.ptr, s.len);
    if (target == 0) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const prefix: []const u8 = "could not find interpreter \"";
        const suffix: []const u8 = "\"";
        const total: u32 = @as(u32, @intCast(prefix.len)) + s.len + @as(u32, @intCast(suffix.len));
        const buf = alloc(total);
        const d: [*]u8 = @ptrFromInt(buf);
        for (prefix, 0..) |b, k| d[k] = b;
        if (s.len > 0) {
            const sp: [*]const u8 = @ptrFromInt(s.ptr);
            for (0..s.len) |k| d[prefix.len + k] = sp[k];
        }
        for (suffix, 0..) |b, k| d[prefix.len + s.len + k] = b;
        const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
        catch_mod.tcl_cmd_error(msg);
    }
    return target;
}

/// ``interp hide path cmd ?hiddenName?``.  ``words[2]`` is the
/// interp path (empty list = current interp; non-empty list resolves
/// to a child interp whose hidden table receives the Command).
/// ``words[3]`` is the source command name.  ``words[4]`` (optional)
/// is the hidden destination name.
pub fn eval_interp_hide(words: []const i32) i32 {
    if (words.len < 4 or words.len > 5) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp hide path cmdName ?hiddenCmdName?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target_interp = resolve_interp_path(words[2]);
    if (target_interp == 0) return 0;
    const src = obj_ensure_string(words[3]);
    const hidden_name = if (words.len >= 5) obj_ensure_string(words[4]) else src;

    // Source namespace: when the target is the caller's own interp
    // we respect an enclosing ``namespace eval`` context (the
    // historical single-interp behaviour); when the target is a
    // different interp the name resolves from that interp's root.
    const src_ns: u32 = if (target_interp == interp_reg.interp_current())
        tcl_ns.ns_current()
    else blk: {
        const t: *interp_reg.Interp = @ptrFromInt(target_interp);
        break :blk t.root_ns;
    };

    const r = hide_mod.hide_command(
        target_interp,
        src_ns,
        src.ptr,
        src.len,
        hidden_name.ptr,
        hidden_name.len,
    );
    switch (r) {
        .ok => return 0,
        .not_found => {
            interp_hide_error("unknown command \"", src.ptr, src.len, "\"");
            return 0;
        },
        .qualified_name_rejected => {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "can't use namespace qualifiers as hidden command token (rename)";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        },
        .hidden_name_taken => {
            interp_hide_error("hidden command named \"", hidden_name.ptr, hidden_name.len, "\" already exists");
            return 0;
        },
    }
}

/// ``interp expose path hiddenName ?newName?``.
pub fn eval_interp_expose(words: []const i32) i32 {
    if (words.len < 4 or words.len > 5) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp expose path hiddenCmdName ?cmdName?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target_interp = resolve_interp_path(words[2]);
    if (target_interp == 0) return 0;
    const hidden_name = obj_ensure_string(words[3]);
    const new_name = if (words.len >= 5) obj_ensure_string(words[4]) else hidden_name;

    // Destination ns mirrors the hide side: enclosing ``namespace
    // eval`` context for same-interp, the child's root for
    // cross-interp.
    const dest_ns: u32 = if (target_interp == interp_reg.interp_current())
        tcl_ns.ns_current()
    else blk: {
        const t: *interp_reg.Interp = @ptrFromInt(target_interp);
        break :blk t.root_ns;
    };

    const r = hide_mod.expose_command(
        target_interp,
        hidden_name.ptr,
        hidden_name.len,
        dest_ns,
        new_name.ptr,
        new_name.len,
    );
    switch (r) {
        .ok => return 0,
        .not_found => {
            interp_hide_error("unknown hidden command \"", hidden_name.ptr, hidden_name.len, "\"");
            return 0;
        },
        .qualified_name_rejected => {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "can not expose to a namespace (use expose to toplevel, then rename)";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        },
        .target_exists => {
            interp_hide_error("exposed command \"", new_name.ptr, new_name.len, "\" already exists");
            return 0;
        },
    }
}

/// ``interp hidden {}`` (and bare ``interp hidden``) — return a Tcl
/// list of hidden-command names.  Walks the interpreter-wide hidden
/// table directly; emission order is bucket-traversal order which
/// isn't stable across grow events but matches ``interp aliases``.
pub fn eval_interp_hidden(words: []const i32) i32 {
    // words[0] = "interp", words[1] = "hidden", words[2] = path
    // (single-interp: always ``{}``).  Reject any trailing args —
    // Tcl 9's ``HiddenCmdsNamesObjCmd`` raises ``wrong # args``
    // for ``objc != 2`` (i.e. requires exactly one arg after the
    // subcommand name).  Our arity check accepts both the bare
    // ``interp hidden`` form and ``interp hidden {}`` since
    // tcltest's top-level emits both shapes depending on the
    // invocation site.
    if (words.len != 2 and words.len != 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp hidden path\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target_interp: u32 = if (words.len == 3) blk: {
        const t = resolve_interp_path(words[2]);
        if (t == 0) return 0;
        break :blk t;
    } else interp_reg.interp_current();
    // Shared walker with ``interp slaves`` / ``interp children`` —
    // same 16-byte bucket layout, same list-quoting contract on
    // the emitted simple names.
    return emit_bucket_names_as_list(
        interp_reg.hidden_table_buf(target_interp),
        interp_reg.hidden_table_cap(target_interp),
    );
}

/// ``interp invokehidden path ?-global? ?-namespace ns? cmd ?arg…?``.
/// Looks ``cmd`` up in the interpreter-wide hidden table and
/// dispatches it with the supplied arguments.  Mirrors Tcl 9's
/// ``InvokeHiddenObjCmd`` (``tclInterp.c``) trimmed to the
/// single-interp scope — ``path`` must be the empty list for us.
///
/// Option flags:
///
/// * ``-global`` — dispatch in the global namespace (root).
/// * ``-namespace ns`` — dispatch in the named namespace.
///
/// With neither flag, the invocation runs in the *global*
/// namespace too (matches C Tcl, where ``InvokeHiddenObjCmd``
/// pushes a fresh call frame with ``FRAME_IS_LAMBDA`` at the
/// root level by default).
///
/// Error shape on miss: ``invalid hidden command name "X"`` —
/// verbatim Tcl 9 wording.  Conflicting ``-global`` +
/// ``-namespace`` raises ``cannot use -global option and
/// -namespace option together``.
// -- ``interp create`` / ``eval`` / ``exists`` / ``slaves`` / ``delete`` --
//
// Minimum-viable child-interpreter primitives.  Mirror of
// ``ChildCreate`` / ``ChildEval`` / ``Tcl_GetChild`` and the
// ``OPT_{CREATE,EVAL,EXISTS,SLAVES,DELETE}`` branches in
// ``InterpObjCmd`` (``tmp/tcl9.0.3/generic/tclInterp.c``).

/// Render an unsigned integer into a bump-allocated decimal
/// string.  Small enough (≤10 digits for u32) to not bother with
/// Tcl's double-pass integer formatter.
pub fn render_uint(value: u32) struct { ptr: u32, len: u32 } {
    var tmp: [12]u8 = undefined;
    var n: u32 = value;
    var pos: u32 = tmp.len;
    if (n == 0) {
        pos -= 1;
        tmp[pos] = '0';
    } else {
        while (n > 0) {
            pos -= 1;
            tmp[pos] = @intCast('0' + (n % 10));
            n /= 10;
        }
    }
    const len: u32 = @intCast(tmp.len - pos);
    const buf = alloc(len);
    const d: [*]u8 = @ptrFromInt(buf);
    for (0..len) |i| d[i] = tmp[pos + i];
    return .{ .ptr = buf, .len = len };
}

/// ``interp create ?-safe? ?--? ?path?``.
///
/// The ``-safe`` bit is recorded on the new Interp's ``flags`` but
/// not enforced (no file / exec / package access to gate — see
/// ``docs/design/runtime/child-interp.md`` §2).  Empty path auto-
/// generates ``interp0`` / ``interp1`` / …; non-empty path may be
/// multi-level (``interp create {a b}`` creates ``b`` as a child of
/// ``a``).  The final path component becomes the new interp's
/// simple name in its parent.
///
/// Option parsing matches the "weird historical rules" in
/// ``InterpObjCmd`` (``tmp/tcl9.0.3/generic/tclInterp.c``):
/// ``-safe`` can appear anywhere up to the path, ``--`` forces
/// the next arg to be treated as a path, and unrecognised leading-
/// dash args raise ``bad option "X": must be -safe or --``.
pub fn eval_interp_create(words: []const i32) i32 {
    var safe_flag: bool = false;
    var saw_dashdash: bool = false;
    var path_obj: i32 = 0;
    var i: u32 = 2;
    while (i < words.len) : (i += 1) {
        const arg = obj_ensure_string(words[i]);
        const ap: [*]const u8 = @ptrFromInt(arg.ptr);
        if (!saw_dashdash and arg.len >= 1 and ap[0] == '-') {
            if (str_eq(ap, arg.len, "-safe")) {
                safe_flag = true;
                continue;
            }
            if (str_eq(ap, arg.len, "--")) {
                saw_dashdash = true;
                continue;
            }
            // Unknown option.  Matches tclsh's wording verbatim.
            const catch_mod = @import("../interp/tcl_catch.zig");
            const prefix: []const u8 = "bad option \"";
            const suffix: []const u8 = "\": must be -safe or --";
            const total: u32 = @as(u32, @intCast(prefix.len)) + arg.len + @as(u32, @intCast(suffix.len));
            const buf = alloc(total);
            const d: [*]u8 = @ptrFromInt(buf);
            for (prefix, 0..) |b, k| d[k] = b;
            if (arg.len > 0) {
                for (0..arg.len) |k| d[prefix.len + k] = ap[k];
            }
            for (suffix, 0..) |b, k| d[prefix.len + arg.len + k] = b;
            const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        if (path_obj != 0) {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "wrong # args: should be \"interp create ?-safe? ?--? ?path?\"";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        path_obj = words[i];
    }

    const current = interp_reg.interp_current();
    var parent_interp: u32 = current;
    var name_ptr: u32 = 0;
    var name_len: u32 = 0;
    if (path_obj == 0) {
        // Auto-generate a fresh name: ``interp<N>`` where N comes
        // from the *parent's* own monotonic counter.  Matches C
        // Tcl's ``Parent.idIssuer`` — kept per-parent so siblings
        // in different parent interps don't collide, and a
        // deleted-then-recreated anonymous interp under parent X
        // doesn't skew the numbering under parent Y.
        //
        // Even with the per-parent counter we still probe for
        // collisions (both against child-registry entries and
        // same-named cmd_table entries in the parent's root ns)
        // because a user may have manually created a proc named
        // ``interp0`` or an explicit child ``interp0`` before
        // reaching the auto-name path.
        const current_i: *interp_reg.Interp = @ptrFromInt(current);
        while (true) {
            const d = render_uint(current_i.id_issuer);
            current_i.id_issuer += 1;
            const prefix: []const u8 = "interp";
            const total: u32 = @as(u32, @intCast(prefix.len)) + d.len;
            const buf = alloc(total);
            const dst: [*]u8 = @ptrFromInt(buf);
            for (prefix, 0..) |b, k| dst[k] = b;
            const dp: [*]const u8 = @ptrFromInt(d.ptr);
            for (0..d.len) |k| dst[prefix.len + k] = dp[k];
            // Also reject collision with an existing command in the
            // parent's ns — matches ``Tcl_GetCommandInfo(interp, buf,
            // &cmdInfo)`` check in C Tcl's ``OPT_CREATE``.
            const parent_root = tcl_ns.ns_root();
            const name_free = interp_reg.child_lookup(current, buf, total) == 0 and
                tcl_ns.ns_cmd_find(parent_root, buf, total) == 0;
            if (name_free) {
                name_ptr = buf;
                name_len = total;
                break;
            }
        }
    } else {
        // Caller-supplied path.  Multi-level paths walk the existing
        // parent chain; the final component becomes the new child's
        // simple name.
        const s = obj_ensure_string(path_obj);
        const elem_count = obj_mod.list_count_elements(s.ptr, s.len);
        if (elem_count <= 1) {
            name_ptr = s.ptr;
            name_len = s.len;
        } else {
            // Walk elements 0..n-2 down the existing chain, element
            // n-1 becomes the new child's name.
            var k: i64 = 0;
            while (k < elem_count - 1) : (k += 1) {
                const elem = obj_mod.list_element_at(s.ptr, s.len, k);
                parent_interp = interp_reg.child_lookup(
                    parent_interp,
                    s.ptr + elem.start,
                    elem.len,
                );
                if (parent_interp == 0) {
                    const catch_mod = @import("../interp/tcl_catch.zig");
                    const prefix: []const u8 = "could not find interpreter \"";
                    const suffix: []const u8 = "\"";
                    const total: u32 = @as(u32, @intCast(prefix.len)) + elem.len + @as(u32, @intCast(suffix.len));
                    const buf = alloc(total);
                    const d: [*]u8 = @ptrFromInt(buf);
                    for (prefix, 0..) |b, kk| d[kk] = b;
                    const pp: [*]const u8 = @ptrFromInt(s.ptr + elem.start);
                    for (0..elem.len) |kk| d[prefix.len + kk] = pp[kk];
                    for (suffix, 0..) |b, kk| d[prefix.len + elem.len + kk] = b;
                    const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
                    catch_mod.tcl_cmd_error(msg);
                    return 0;
                }
            }
            const final_elem = obj_mod.list_element_at(s.ptr, s.len, elem_count - 1);
            name_ptr = s.ptr + final_elem.start;
            name_len = final_elem.len;
        }
        if (interp_reg.child_lookup(parent_interp, name_ptr, name_len) != 0) {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const prefix: []const u8 = "interpreter named \"";
            const suffix: []const u8 = "\" already exists, cannot create";
            const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
            const buf = alloc(total);
            const d: [*]u8 = @ptrFromInt(buf);
            for (prefix, 0..) |b, k| d[k] = b;
            const np: [*]const u8 = @ptrFromInt(name_ptr);
            for (0..name_len) |k| d[prefix.len + k] = np[k];
            for (suffix, 0..) |b, k| d[prefix.len + name_len + k] = b;
            const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
    }

    const flags: u32 = if (safe_flag) interp_reg.INTERP_SAFE else 0;
    const child = interp_reg.child_create(parent_interp, name_ptr, name_len, flags);

    // Register a ``CMD_INTERP_CHILD`` Command in the parent interp's
    // root ns so ``<child> eval script`` resolves through the
    // regular ``proc_lookup`` dispatcher.  The Command stashes
    // ``child`` in its ``params_obj`` slot; ``dispatch_interp_child``
    // reads it back on invocation.  Matches C Tcl where
    // ``Tcl_CreateSlaveCmd`` installs the interp command in the
    // parent interp's global ns.
    const parent_i: *interp_reg.Interp = @ptrFromInt(parent_interp);
    const cmd = interp_reg.alloc_child_command(
        parent_i.root_ns,
        name_ptr,
        name_len,
        child,
    );
    _ = tcl_ns.ns_cmd_put(parent_i.root_ns, name_ptr, name_len, cmd);
    procs.proc_count_bump();
    procs.lru_invalidate_all();

    // Return the path as supplied (or the auto-generated simple
    // name) — matches tclsh's ``Tcl_SetObjResult(interp, childPtr)``
    // on OPT_CREATE.
    if (path_obj != 0) return words_obj_new_string_dup(
        obj_ensure_string(path_obj).ptr,
        obj_ensure_string(path_obj).len,
    );
    return words_obj_new_string_dup(name_ptr, name_len);
}

/// ``interp eval path script ?script ...?``.  Concatenate scripts
/// with a single-space separator (matches ``InterpEvalObjCmd``'s
/// ``Tcl_ConcatObj`` call) and eval the result inside the resolved
/// interp's root namespace.
/// ``interp exists ?path?``.  Empty or missing path returns 1
/// (the current interp always exists); non-empty path returns 1
/// if it resolves, 0 if not.  Unlike ``eval`` and the others,
/// ``exists`` never raises — a missing path is a normal ``0``
/// return (matching ``OPT_EXISTS`` in ``tclInterp.c``).
pub fn eval_interp_exists(words: []const i32) i32 {
    if (words.len > 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp exists ?path?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    if (words.len == 2) return obj_new_int(1);
    const path = obj_ensure_string(words[2]);
    if (path.len == 0) return obj_new_int(1);
    const target = interp_reg.resolve_path(interp_reg.interp_current(), path.ptr, path.len);
    return obj_new_int(if (target != 0) 1 else 0);
}

/// ``interp slaves ?path?`` (and the Tcl 9 alias ``interp children``).
/// Returns a Tcl list of the (direct) child names under the resolved
/// interp — simple names, not full paths.
pub fn eval_interp_slaves(words: []const i32) i32 {
    if (words.len > 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp slaves ?path?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target: u32 = if (words.len == 3) blk: {
        const t = resolve_interp_path(words[2]);
        if (t == 0) return 0;
        break :blk t;
    } else interp_reg.interp_current();

    const t_ptr: *interp_reg.Interp = @ptrFromInt(target);
    return emit_bucket_names_as_list(t_ptr.children.buf, t_ptr.children.cap);
}

/// Produce a canonical Tcl list of the populated simple names in a
/// ``hash_table.Table(16)``-shaped bucket array.  Used by
/// ``interp slaves`` / ``interp children`` / ``interp hidden`` /
/// ``interp aliases``: each consults a different bucket array but
/// the output shape (space-separated, list-quoted on any element
/// that contains whitespace / braces / backslashes / leading ``#``)
/// is identical across them.
///
/// Bucket layout (see ``hash_table.zig``):
///
///     [0..3]   name_ptr  (0 = empty slot)
///     [4..7]   name_len
///     [8..11]  hash
///     [12..15] value    (0 = tombstoned)
///
/// Two-pass: size (worst-case ``2 * name_len + 2`` per element per
/// ``list_elem_quote``, plus one byte of separator) then fill using
/// :func:`tcl_list_quote.list_elem_quote` on element 0 and
/// :func:`list_elem_quote_nth` on the rest so a leading ``#`` on
/// element > 0 stays unbraced (matches ``UpdateStringOfList``).
pub fn emit_bucket_names_as_list(buf_addr: u32, cap: u32) i32 {
    if (buf_addr == 0 or cap == 0) return obj_new_string(0, 0);
    const bucket_size: u32 = 16;
    const list_quote = @import("../valtypes/tcl_list_quote.zig");

    var total: u32 = 0;
    var count: u32 = 0;
    var i: u32 = 0;
    while (i < cap) : (i += 1) {
        const bucket = buf_addr + i * bucket_size;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) continue;
        const handle: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (handle == 0) continue; // tombstoned
        const nlen: u32 = @bitCast(read_i32(bucket + 4));
        if (count > 0) total += 1;
        // Worst-case expansion from ``list_elem_quote``: ``2n + 2``.
        // Empty names still need at least ``{}`` (2 bytes).
        total += if (nlen == 0) 2 else (2 * nlen + 2);
        count += 1;
    }
    if (total == 0) return obj_new_string(0, 0);

    const out = alloc(total);
    var off: u32 = 0;
    count = 0;
    i = 0;
    while (i < cap) : (i += 1) {
        const bucket = buf_addr + i * bucket_size;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) continue;
        const handle: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (handle == 0) continue;
        const nlen: u32 = @bitCast(read_i32(bucket + 4));
        if (count > 0) {
            const d: [*]u8 = @ptrFromInt(out + off);
            d[0] = ' ';
            off += 1;
        }
        off = if (count == 0)
            list_quote.list_elem_quote(out, off, ep, nlen)
        else
            list_quote.list_elem_quote_nth(out, off, ep, nlen);
        count += 1;
    }
    return obj_new_string(@bitCast(out), @bitCast(off));
}

/// ``interp delete ?path ...?``.  Each path is resolved and
/// unlinked from its parent's children registry.  C Tcl does a
/// full teardown (clears the child's cmd_table, hidden table,
/// cascades into namespace deletion); we trim to the unlink —
/// the child's byte regions stay live in bump memory but are
/// unreachable from the parent, so they can't be dispatched
/// again.  See ``docs/design/runtime/child-interp.md`` §9 for
/// the deferred-cascade discussion.
///
/// Deleting the current interp (path = ``{}``) raises the same
/// ``cannot delete the current interpreter`` error C Tcl does.
pub fn eval_interp_delete(words: []const i32) i32 {
    if (words.len < 2) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp delete ?path ...?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    var k: u32 = 2;
    while (k < words.len) : (k += 1) {
        const path = obj_ensure_string(words[k]);
        if (path.len == 0) {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "cannot delete the current interpreter";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        const target = interp_reg.resolve_path(interp_reg.interp_current(), path.ptr, path.len);
        if (target == 0) {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const prefix: []const u8 = "could not find interpreter \"";
            const suffix: []const u8 = "\"";
            const total: u32 = @as(u32, @intCast(prefix.len)) + path.len + @as(u32, @intCast(suffix.len));
            const buf = alloc(total);
            const d: [*]u8 = @ptrFromInt(buf);
            for (prefix, 0..) |b, j| d[j] = b;
            const sp: [*]const u8 = @ptrFromInt(path.ptr);
            for (0..path.len) |j| d[prefix.len + j] = sp[j];
            for (suffix, 0..) |b, j| d[prefix.len + path.len + j] = b;
            const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
        // Find the direct parent by walking the chain — single-level
        // paths land in the current interp; multi-level paths resolve
        // to the inner-most child so its parent is the next-outer
        // component.  We read the child's ``parent`` field rather
        // than re-walking — cheaper and always correct.
        const t: *interp_reg.Interp = @ptrFromInt(target);
        const parent = t.parent;
        if (parent != 0 and t.name_len > 0) {
            // Tombstone the ``<child>`` Command from the parent's
            // root ns so ``<child> eval {...}`` post-delete raises
            // "unknown command" rather than dispatching into a
            // deleted interp.
            const parent_i: *interp_reg.Interp = @ptrFromInt(parent);
            _ = tcl_ns.ns_cmd_clear(parent_i.root_ns, t.name_ptr, t.name_len);
            _ = interp_reg.child_delete(parent, t.name_ptr, t.name_len);
        }
        // Flush the LRU — a parent alias that targets a command in
        // the deleted child would otherwise stay cached.
        procs.lru_invalidate_all();
    }
    return 0;
}

/// ``interp target path alias`` — not supported this wave, but we
/// emit the canonical tclsh arity error on the no-arg invocation
/// instead of falling through to the unknown-subcommand stub
/// (tclsh's ``InterpObjCmd`` validates this branch before the
/// generic error-dispatch path).  Matches
/// ``tmp/tcl9.0.3/generic/tclInterp.c`` ``OPT_TARGET`` arg-count
/// check.  With the right number of args we still return the
/// "unsupported" stub for now.
pub fn eval_interp_target(words: []const i32) i32 {
    if (words.len != 4) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp target path alias\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // Arity is valid but the operation isn't wired up.  Surface
    // the unsupported stub rather than lying with a bogus result.
    const stubs = @import("../stubs/tcl_stubs.zig");
    stubs.unsupported_sub("interp", "target");
    return 0;
}

/// ``interp issafe ?path?`` — read the ``INTERP_SAFE`` flag on the
/// resolved interp.  The flag is set at creation time via
/// ``interp create -safe`` but carries no gating consequences in
/// this runtime (see ``docs/design/runtime/child-interp.md`` §2).
pub fn eval_interp_issafe(words: []const i32) i32 {
    if (words.len > 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp issafe ?path?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target: u32 = if (words.len == 3) blk: {
        const t = resolve_interp_path(words[2]);
        if (t == 0) return 0;
        break :blk t;
    } else interp_reg.interp_current();
    const t: *interp_reg.Interp = @ptrFromInt(target);
    return obj_new_int(if ((t.flags & interp_reg.INTERP_SAFE) != 0) 1 else 0);
}
// -- ``namespace which`` ---------------------------------------------------

/// Render a Command's current fully-qualified name.  Reads the
/// Command's stored name slot directly — ``rename`` and ``expose``
/// both keep it in sync with the user-visible identity (compiled
/// procs' sidecar preserves the WASM export name separately; the
/// stored slot is always the live FQN).
pub fn command_fqn_obj(cmd: u32) i32 {
    const name_ptr: u32 = @bitCast(read_i32(cmd));
    const name_len: u32 = @bitCast(read_i32(cmd + 4));
    if (name_len == 0) return obj_new_string(0, 0);
    return obj_new_string(@bitCast(name_ptr), @bitCast(name_len));
}

/// Render the FQN of a namespace variable: ``<ns_full>::<name>``
/// with the root-ns ``::`` collapsed so a root-level variable
/// reads as ``::x`` rather than ``::::x``.  Matches
/// :func:`tcl_ns.ns_build_fqn`.
pub fn variable_fqn_obj(ns: u32, simple_ptr: u32, simple_len: u32) i32 {
    const fqn = tcl_ns.ns_build_fqn(ns, simple_ptr, simple_len);
    return obj_new_string(@bitCast(fqn.ptr), @bitCast(fqn.len));
}

pub fn eval_namespace_which(words: []const i32) i32 {
    // Argument shapes:
    //   namespace which name                         → -command (default)
    //   namespace which -command name
    //   namespace which -variable name
    // Anything else is a plain miss — return empty.  We don't raise
    // wrong-args here to stay consistent with the "probe, don't
    // error" philosophy of C Tcl's ``Tcl_NamespaceWhichObjCmd``.
    if (words.len < 3 or words.len > 4) return obj_new_string(0, 0);

    var which_variable = false;
    var name_idx: u32 = 2;
    if (words.len == 4) {
        const flag = obj_ensure_string(words[2]);
        const fp: [*]const u8 = @ptrFromInt(flag.ptr);
        if (str_eq(fp, flag.len, "-variable")) {
            which_variable = true;
        } else if (!str_eq(fp, flag.len, "-command")) {
            return obj_new_string(0, 0);
        }
        name_idx = 3;
    }
    const name = obj_ensure_string(words[name_idx]);
    if (name.len == 0) return obj_new_string(0, 0);

    if (which_variable) {
        // Variable resolution: qualified names walk the ns tree to
        // the target ns + simple name, unqualified names check the
        // current ns only (C Tcl doesn't walk ``namespace path``
        // for variables — the path is commands-only).
        const cxt = tcl_ns.ns_current();
        const r = tcl_ns.ns_resolve_qualified(cxt, name.ptr, name.len);
        if (r.simple_len == 0) return obj_new_string(0, 0);
        if (r.target_ns != 0) {
            const v = tcl_ns.ns_var_find(r.target_ns, r.simple_ptr, r.simple_len);
            if (v != 0) return variable_fqn_obj(r.target_ns, r.simple_ptr, r.simple_len);
        }
        if (r.alt_ns != 0) {
            const v = tcl_ns.ns_var_find(r.alt_ns, r.simple_ptr, r.simple_len);
            if (v != 0) return variable_fqn_obj(r.alt_ns, r.simple_ptr, r.simple_len);
        }
        return obj_new_string(0, 0);
    }

    // Command resolution: reuse the canonical ``ns_find_command``
    // walker so the lookup path is identical to proc dispatch.  The
    // Command's live name slot is the FQN we return — imports are
    // NOT unwrapped (matches C Tcl's rule that ``namespace which``
    // on an imported command returns the redirect's FQN, not the
    // source's).
    const cxt = tcl_ns.ns_current();
    const cmd = tcl_ns.ns_find_command(cxt, name.ptr, name.len);
    if (cmd == 0) return obj_new_string(0, 0);
    return command_fqn_obj(cmd);
}
