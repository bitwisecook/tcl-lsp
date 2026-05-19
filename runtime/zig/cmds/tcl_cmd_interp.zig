// Tcl command management: interp, rename, namespace-which.
//
// Implements the ``interp``, ``rename``, and ``namespace which``
// command groups.  These do not call eval_script or eval_command,
// so they can be imported from tcl_interp.zig without a cycle.
// Renamed from tcl_interp_interp.zig (the old name was misleading).

const std = @import("std");

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
            .alias_loop => return 0, // unreachable on delete form
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
        .ok => {
            // Rewrite the AliasRec's token slot when an alias is
            // renamed via the move form *and* the original token
            // hasn't been preserved by the chain.  In C Tcl, the
            // ``aliasTable`` keys by the token so a rename leaves
            // the token intact — we mirror that by keeping the
            // ``token_*`` slot untouched; the FQN on the Command
            // header is the only thing that changes.  No extra
            // work needed here.
            return 0;
        },
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
        .alias_loop => {
            // Alias rename would form a dispatch cycle —
            // ``cannot define or rename alias "<NEW>": would
            // create a loop`` (interp-17.4 / 17.5 / 17.6).
            raise_alias_loop_error(new_r.simple_ptr, new_r.simple_len);
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
/// Raise the canonical Tcl 9 alias-loop diagnostic.  Used by
/// :func:`interp_alias_create` when its pre-check detects that
/// installing the alias would form a cycle that ``dispatch_alias``
/// would later recurse on forever.
fn raise_alias_loop_error(new_name_ptr: u32, new_name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix: []const u8 = "cannot define or rename alias \"";
    const suffix: []const u8 = "\": would create a loop";
    const total: u32 = @as(u32, @intCast(prefix.len)) + new_name_len + @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |b, k| d[k] = b;
    if (new_name_len > 0) {
        const np: [*]const u8 = @ptrFromInt(new_name_ptr);
        for (0..new_name_len) |k| d[prefix.len + k] = np[k];
    }
    for (suffix, 0..) |b, k| d[prefix.len + new_name_len + k] = b;
    const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}

/// Detect whether installing an alias ``new`` in ``child_interp``
/// targeting ``target`` in ``target_interp`` would form a dispatch
/// cycle that ``dispatch_alias`` would later recurse on forever.
///
/// Mirrors ``AliasCheckLoop`` in ``tclInterp.c``: walk the target
/// chain through whatever interps the aliases route across,
/// stopping when we either bottom out at a non-alias command or
/// hit the (interp, simple_name) pair of the new alias.  Capped
/// at ``MAX_LOOP_HOPS`` so the walk terminates on hand-built
/// mutual chains.  Exhausting the cap is treated as a loop so a
/// pathologically deep chain can't slip past detection here and
/// then recurse at dispatch (Copilot review on PR #451).
fn alias_loop_would_form(
    child_interp: u32,
    new_name_ptr: u32,
    new_name_len: u32,
    target_interp: u32,
    target_name_ptr: u32,
    target_name_len: u32,
) bool {
    if (new_name_len == 0 or target_name_len == 0) return false;
    const new_simple = simple_name_of(new_name_ptr, new_name_len);
    var cur_interp = target_interp;
    var cur_name_ptr: u32 = target_name_ptr;
    var cur_name_len: u32 = target_name_len;
    const MAX_LOOP_HOPS: u32 = 64;
    var hops: u32 = 0;
    while (hops < MAX_LOOP_HOPS) : (hops += 1) {
        if (cur_interp == 0) return false;
        const cur_simple = simple_name_of(cur_name_ptr, cur_name_len);
        // Hit the proposed new alias's (interp, simple_name) →
        // loop.  Both must match — same name in a different interp
        // is a different command.
        if (cur_interp == child_interp and cur_simple.len == new_simple.len) {
            const np: [*]const u8 = @ptrFromInt(new_simple.ptr);
            const cp: [*]const u8 = @ptrFromInt(cur_simple.ptr);
            var same = true;
            var k: u32 = 0;
            while (k < new_simple.len) : (k += 1) {
                if (np[k] != cp[k]) {
                    same = false;
                    break;
                }
            }
            if (same) return true;
        }
        // Walk one alias hop: look the current command up in its
        // host interp's namespace tree.  Non-alias means the chain
        // bottoms out at a regular command (proc / builtin) — no
        // loop possible.
        const ci: *interp_reg.Interp = @ptrFromInt(cur_interp);
        const cmd = tcl_ns.ns_find_command(ci.root_ns, cur_simple.ptr, cur_simple.len);
        if (cmd == 0 or !alias_mod.is_alias(cmd)) return false;
        const rec = alias_mod.alias_rec(cmd);
        if (rec.target_name_len == 0) return false;
        // ``parent_interp == 0`` is the same-interp shortcut C Tcl
        // uses when an alias's target lives in the same interp as
        // the alias itself; otherwise it names the target's host.
        cur_interp = if (rec.parent_interp == 0) cur_interp else rec.parent_interp;
        cur_name_ptr = rec.target_name_ptr;
        cur_name_len = rec.target_name_len;
    }
    // Hop cap exhausted without bottoming out at a non-alias —
    // treat as a loop so a deep chain can't escape detection
    // here and recurse at dispatch (Copilot review).
    return true;
}

/// Return the simple (un-qualified) name span of *name*.  Strips
/// every ``::`` separator from the end backwards, returning the
/// trailing component.  ``::foo::bar`` → ``bar``; bare ``foo`` →
/// ``foo``.  Used only for the loop-check comparator so callers
/// can compare across the qualified / unqualified spellings the
/// alias machinery accepts at the user-facing surface.
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
    // Alias-loop pre-check.  Walk the proposed target chain
    // through the (possibly cross-interp) alias graph and refuse
    // to install the alias if it would create a cycle.  Mirrors
    // ``AliasCreate`` in ``tclInterp.c`` which calls
    // ``AliasCheckLoop`` before storing the alias (interp-17.1 /
    // 17.2 / 17.3).
    if (alias_loop_would_form(
        child_interp,
        new_name_ptr,
        new_name_len,
        parent_interp,
        target_name_ptr,
        target_name_len,
    )) {
        raise_alias_loop_error(new_name_ptr, new_name_len);
        return 0;
    }
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

    // If a Command already lives under the destination simple name
    // (in r.target_ns), the ``ns_cmd_put`` below overwrites it.
    // Mirror C Tcl's ``Tcl_CreateObjCommand`` deleteProc which
    // fires the old alias's ``AliasObjCmdDeleteProc`` cleanup
    // (removes its ``aliasTable`` entry).  In our model, we
    // explicitly clear the old AliasRec's chain hookup so
    // ``interp aliases`` doesn't double-report.
    //
    // Special case: when the existing Command is a
    // ``CMD_INTERP_CHILD`` (e.g. ``interp create a`` registered
    // ``a`` as the child-interp dispatch command), overwriting
    // it means the child interp gets deleted as a side effect
    // — C Tcl's deleteProc fires ``ChildObjCmdDeleteProc`` which
    // calls ``Tcl_DeleteInterp``.  Mirror that here so a later
    // dispatch through the alias's stashed parent_interp picks
    // up the ``INTERP_DELETED`` flag and surfaces the canonical
    // ``cannot define or rename alias "X": interpreter deleted``
    // diagnostic (interp-14.4).
    const existing_cmd = tcl_ns.ns_cmd_find(r.target_ns, r.simple_ptr, r.simple_len);
    if (existing_cmd != 0) {
        const existing_flags: u32 = @bitCast(obj_mod.read_i32(existing_cmd + procs.OFF_FLAGS));
        if ((existing_flags & 0x200) != 0) {
            // CMD_INTERP_CHILD — extract the embedded child interp
            // and tear it down before the alias install.  Match
            // the full ``interp delete`` cascade: drop xlinks,
            // mark deleted via ``child_delete``, and clear the
            // parent's cmd_table bucket so ``[info commands]``
            // no longer reports the dispatch command.
            const victim = interp_reg.cmd_child_interp(existing_cmd);
            if (victim != 0) {
                const v: *interp_reg.Interp = @ptrFromInt(victim);
                if (v.parent != 0 and v.name_len > 0) {
                    const parent_i: *interp_reg.Interp = @ptrFromInt(v.parent);
                    const xlinks = @import("../interp/tcl_xlinks.zig");
                    xlinks.drop_for_interp(victim);
                    _ = interp_reg.child_delete(v.parent, v.name_ptr, v.name_len);
                    _ = tcl_ns.ns_cmd_clear(parent_i.root_ns, v.name_ptr, v.name_len);
                }
            }
        }
    }
    if (alias_mod.is_alias(existing_cmd)) {
        alias_mod.alias_clear(existing_cmd);
    }
    // If the alias target now lives in a deleted interp, refuse
    // the create — mirror ``AliasCreate``'s ``Tcl_InterpDeleted``
    // check after the loop probe (tclInterp.c lines 1410-1420).
    if (parent_interp != 0 and interp_reg.is_deleted(parent_interp)) {
        interp_reg.leave(save);
        const catch_mod = @import("../interp/tcl_catch.zig");
        const prefix: []const u8 = "cannot define or rename alias \"";
        const suffix: []const u8 = "\": interpreter deleted";
        const total: u32 = @as(u32, @intCast(prefix.len)) + new_name_len + @as(u32, @intCast(suffix.len));
        const buf = alloc(total);
        const d: [*]u8 = @ptrFromInt(buf);
        for (prefix, 0..) |b, k| d[k] = b;
        if (new_name_len > 0) {
            const np: [*]const u8 = @ptrFromInt(new_name_ptr);
            for (0..new_name_len) |k| d[prefix.len + k] = np[k];
        }
        for (suffix, 0..) |b, k| d[prefix.len + new_name_len + k] = b;
        const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // Token collision: if the proposed token already names an
    // alias in the child's aliases chain (from a previously-renamed
    // alias whose source Command moved off the original name slot
    // but kept its token), prepend ``::`` until the token is
    // unique.  Mirrors ``TclAliasCreate`` in tclInterp.c lines
    // 1553-1580 ("alias name cannot be used as unique token, it
    // is already taken — produce a unique token by prepending '::'
    // repeatedly").  The prepended token is what
    // ``interp aliases`` will report for the new alias.
    var token_ptr: u32 = new_name_ptr;
    var token_len: u32 = new_name_len;
    while (interp_reg.alias_chain_find_token(child_interp, .child, token_ptr, token_len) != 0) {
        const prefix_len: u32 = 2;
        const new_len: u32 = prefix_len + token_len;
        const new_buf = alloc(new_len);
        const dst: [*]u8 = @ptrFromInt(new_buf);
        dst[0] = ':';
        dst[1] = ':';
        if (token_len > 0) memcpy(new_buf + prefix_len, token_ptr, token_len);
        token_ptr = new_buf;
        token_len = new_len;
    }

    //
    // The parent interp is stashed directly on the ``AliasRec``
    // (via ``alias_alloc``'s last parameter) rather than on the
    // Command's ``OFF_IMPORT_REF_HEAD`` slot: that slot is shared
    // with the ``namespace import`` back-reference list head, so a
    // later ``namespace import`` of this alias could overwrite the
    // stashed Interp* and corrupt cross-interp dispatch.  Zero
    // here means "same-interp dispatch; no swap needed".
    const stash_parent: u32 = if (parent_interp != child_interp) parent_interp else 0;
    const cmd = alias_mod.alias_alloc_with_token(
        r.target_ns,
        r.simple_ptr,
        r.simple_len,
        token_ptr,
        token_len,
        target_name_ptr,
        target_name_len,
        n_prefix,
        prefix_buf,
        stash_parent,
        child_interp,
    );
    _ = tcl_ns.ns_cmd_put(r.target_ns, r.simple_ptr, r.simple_len, cmd);
    interp_reg.leave(save);
    // Bump the proc counter so ``proc_buf_nonzero`` fires for
    // bundles whose only commands are aliases.
    procs.proc_count_bump();
    return words_obj_new_string_dup(new_name_ptr, new_name_len);
}

pub fn interp_alias_query(child_interp: u32, new_name_ptr: u32, new_name_len: u32) i32 {
    // Look up the alias by its original token (the simple name the
    // caller passed to ``interp alias``).  Survives ``rename`` of
    // the source Command in the namespace tree because the token
    // is heap-copied onto the :type:`AliasRec`.  Matches C Tcl's
    // ``Tcl_FindHashEntry(&iPtr->child.aliasTable, ...)``.
    const rec = interp_reg.alias_chain_find_token(
        child_interp,
        .child,
        new_name_ptr,
        new_name_len,
    );
    if (rec == 0) return 0;
    return alias_mod.alias_describe_rec(rec);
}

pub fn interp_alias_delete(child_interp: u32, new_name_ptr: u32, new_name_len: u32) i32 {
    // Resolve by token first — covers the ``interp alias child
    // foo {}`` form even after a ``rename foo zop`` has moved the
    // source Command.  ``Tcl_DeleteAlias`` in tclInterp.c keeps the
    // hashtable entry keyed by the original name for the same
    // reason.
    const rec = interp_reg.alias_chain_find_token(
        child_interp,
        .child,
        new_name_ptr,
        new_name_len,
    );
    if (rec == 0) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const prefix: []const u8 = "alias \"";
        const suffix: []const u8 = "\" not found";
        const total: u32 = @as(u32, @intCast(prefix.len)) + new_name_len + @as(u32, @intCast(suffix.len));
        const buf = alloc(total);
        const d: [*]u8 = @ptrFromInt(buf);
        for (prefix, 0..) |b, k| d[k] = b;
        if (new_name_len > 0) {
            const np: [*]const u8 = @ptrFromInt(new_name_ptr);
            for (0..new_name_len) |k| d[prefix.len + k] = np[k];
        }
        for (suffix, 0..) |b, k| d[prefix.len + new_name_len + k] = b;
        const msg = rt.obj_new_string(@bitCast(buf), @bitCast(total));
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // Now find the live Command — its name slot may have been
    // ``rename``d off the original token.  Walk the AliasRec's
    // ``parent_interp`` / ``child_interp`` slots through the
    // interp's namespace tree and clear the bucket that still
    // points at this alias.
    const child: *interp_reg.Interp = @ptrFromInt(child_interp);
    const swapped_child = child_interp != interp_reg.interp_current();
    const save = if (swapped_child) interp_reg.enter(child_interp) else interp_reg.EnterSave{
        .prev_interp = interp_reg.current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    // The Command's current name lives on its header; walk the
    // child interp's root ns for a matching alias-flagged entry.
    const cmd = alias_mod.find_command_by_rec(child.root_ns, rec);
    if (cmd != 0) {
        const r2 = alias_mod.alias_rec(cmd);
        // Read the Command's stored name from its header and strip
        // any ``::`` prefix — alias Commands renamed into a
        // namespace store the FQN on the header, but
        // ``ns_cmd_clear`` keys buckets by the simple name (Codex /
        // Copilot review on PR #451).  ``ns_cmd_clear`` tombstones
        // the bucket without freeing — the Command and AliasRec
        // stay around in bump memory, but ``alias_clear`` below
        // also un-links them from the per-interp chains so
        // ``interp aliases`` no longer sees them.
        const hdr_ptr: u32 = @bitCast(obj_mod.read_i32(cmd));
        const hdr_len: u32 = @bitCast(obj_mod.read_i32(cmd + 4));
        const simple = simple_name_of(hdr_ptr, hdr_len);
        // Walk the namespace tree starting at the child's root for
        // a bucket holding ``cmd``.  Use the live-name spelling
        // (post-rename).
        _ = tcl_ns.ns_cmd_clear(alias_mod.alias_host_ns(child.root_ns, cmd), simple.ptr, simple.len);
        _ = r2;
    }
    alias_mod.alias_clear_rec(rec);
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
///
/// Walks the per-interp aliases linked list (``Interp.aliases_head``)
/// which is keyed by the original token — so a ``rename foo zop``
/// of an alias's source Command doesn't change what ``interp
/// aliases`` reports.  Matches C Tcl's ``InterpAliasesCmd`` which
/// iterates ``Interp.child.aliasTable``.
pub fn interp_aliases_list_for(target_interp: u32) i32 {
    if (target_interp == 0) return obj_new_string(0, 0);
    const t: *interp_reg.Interp = @ptrFromInt(target_interp);

    // Two-pass build: size, then fill.  ``list_elem_quote`` /
    // ``_nth`` write at most ``2*len + 2`` bytes per element plus
    // one separator each — same budget as the previous
    // walk_tree_cmd_tables-based implementation.
    var total: u32 = 0;
    var count: u32 = 0;
    var cur: u32 = t.aliases_head;
    while (cur != 0) {
        const r: *alias_mod.AliasRec = @ptrFromInt(cur);
        if (r.token_len > 0 or count > 0) {
            // Skip cleared records (token_len was zeroed by
            // alias_clear).  Live records always have token_len
            // > 0 since the original alias name was at least one
            // byte.
            if (r.token_len > 0) {
                if (count > 0) total += 1;
                total += 2 * r.token_len + 2;
                count += 1;
            }
        }
        cur = r.next_in_child;
    }
    if (count == 0) return obj_new_string(0, 0);

    const buf = alloc(total);
    var off: u32 = 0;
    var written: u32 = 0;
    cur = t.aliases_head;
    const list_quote = @import("../valtypes/tcl_list_quote.zig");
    while (cur != 0) {
        const r: *alias_mod.AliasRec = @ptrFromInt(cur);
        if (r.token_len > 0) {
            if (written > 0) {
                const d: [*]u8 = @ptrFromInt(buf + off);
                d[0] = ' ';
                off += 1;
            }
            off = if (written == 0)
                list_quote.list_elem_quote(buf, off, r.token_ptr, r.token_len)
            else
                list_quote.list_elem_quote_nth(buf, off, r.token_ptr, r.token_len);
            written += 1;
        }
        cur = r.next_in_child;
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
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
    // Safe-interpreter gate: a safe interp cannot ``interp hide``
    // its own commands or those of any descendant.  Matches
    // ``HiddenObjCmd`` in tclInterp.c which raises
    // ``permission denied: safe interpreter cannot hide commands``
    // before the path is resolved.
    if (interp_reg.interp_is_safe(interp_reg.interp_current())) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "permission denied: safe interpreter cannot hide commands";
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
            const err_text = "cannot use namespace qualifiers in hidden command token (rename)";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        },
        .non_global_source => {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "can only hide global namespace commands (use rename then hide)";
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
    // Safe-interpreter gate: a safe interp cannot ``interp expose``
    // hidden commands.  Matches ``ExposeObjCmd`` in tclInterp.c.
    if (interp_reg.interp_is_safe(interp_reg.interp_current())) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "permission denied: safe interpreter cannot expose commands";
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

    // Inherit the ``INTERP_SAFE`` bit from the parent: child interps
    // of a safe parent are themselves safe regardless of whether the
    // creator passed ``-safe``.  Matches C Tcl's ``ChildCreate``,
    // which copies the parent's safety state onto the new child
    // (``tmp/tcl9.0.3/generic/tclInterp.c`` ``ChildCreate`` —
    // ``childIPtr->flags |= Tcl_IsSafe(parentInterp) ? SAFE_INTERP : 0``).
    var flags: u32 = if (safe_flag) interp_reg.INTERP_SAFE else 0;
    if (interp_reg.interp_is_safe(parent_interp)) {
        flags |= interp_reg.INTERP_SAFE;
    }
    const child = interp_reg.child_create(parent_interp, name_ptr, name_len, flags);

    // Inherit the parent's recursion limit so ``interp create`` inside
    // a parent that's already adjusted its limit (e.g. test 29.4.1's
    // ``interp recursionlimit {} 50`` followed by ``interp create``)
    // produces a child seeded with the parent's value rather than the
    // default.  Mirrors C Tcl's ``ChildCreate`` which copies
    // ``parent->maxNestingDepth`` onto ``child->maxNestingDepth``.
    const parent_limit = interp_reg.interp_recursion_limit(parent_interp);
    interp_reg.interp_set_recursion_limit(child, parent_limit);

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

    // Seed the standard ``tcl_*`` globals in the freshly-created
    // child so ``[info globals]`` and ``[array names tcl_platform]``
    // return the C-Tcl-equivalent shape on first use.  Real Tcl
    // runs ``Tcl_Init`` (trusted) or ``MakeSafe`` (safe) which both
    // populate these.  Without this seeding, safe-6.1 / safe-6.3
    // see an empty globals list and fail.  Use the seeder helper
    // so the same vars are seeded for both safe and trusted
    // children — safe-6.x specifies the trusted-equivalent set
    // intentionally redacted (no ``machine`` / ``os`` / ``user``
    // when the interp is safe; ``MakeSafe`` strips those).
    //
    // Use the *effective* safety state on the child (which may
    // have inherited ``INTERP_SAFE`` from the parent) rather than
    // the raw ``-safe`` flag — Codex review on PR #451 caught a
    // path where a safe parent's child without ``-safe`` got the
    // trusted ``tcl_platform`` seed despite being marked safe.
    seed_child_globals(child, (flags & interp_reg.INTERP_SAFE) != 0);

    // Return the path as supplied (or the auto-generated simple
    // name) — matches tclsh's ``Tcl_SetObjResult(interp, childPtr)``
    // on OPT_CREATE.
    if (path_obj != 0) return words_obj_new_string_dup(
        obj_ensure_string(path_obj).ptr,
        obj_ensure_string(path_obj).len,
    );
    return words_obj_new_string_dup(name_ptr, name_len);
}

/// Populate the standard ``tcl_*`` globals in a fresh child interp
/// so ``[info globals]`` / ``[array names tcl_platform]`` produce
/// the Tcl-equivalent shape from the first command.  Mirrors
/// ``Tcl_Init`` (trusted children) and ``MakeSafe`` (safe children)
/// in ``tmp/tcl9.0.3/generic/tclInterp.c``.
///
/// ``MakeSafe`` redacts the unsafe ``tcl_platform`` elements
/// (``os``, ``osVersion``, ``machine``, ``user``) so a safe child
/// only sees ``byteOrder``, ``engine``, ``pathSeparator``,
/// ``platform``, ``pointerSize``, ``threaded``, ``wordSize``.
fn seed_child_globals(child: u32, safe_flag: bool) void {
    // Run the seed as a Tcl script inside the child so it threads
    // through the normal ``global_set`` / ``array set`` paths and
    // lands in the child's namespace state regardless of the
    // caller's outer frame context.  Mirrors C Tcl's ``Tcl_Init``
    // / ``MakeSafe`` which evaluate :file:`init.tcl` (or its safe
    // subset) in the new interp.  The exact text is fixed at
    // compile time so this stays allocation-free past the script
    // pointer.
    const seed_trusted: []const u8 =
        \\set ::tcl_interactive 0
        \\set ::tcl_patchLevel 9.0.3
        \\set ::tcl_version 9.0
        \\array set ::tcl_platform {byteOrder littleEndian engine Tcl pathSeparator : platform unix pointerSize 4 threaded 0 wordSize 8 machine wasm32 os Linux osVersion 0.0 user wasm}
    ;
    const seed_safe: []const u8 =
        \\set ::tcl_interactive 0
        \\set ::tcl_patchLevel 9.0.3
        \\set ::tcl_version 9.0
        \\array set ::tcl_platform {byteOrder littleEndian engine Tcl pathSeparator : platform unix pointerSize 4 threaded 0 wordSize 8}
    ;
    const script: []const u8 = if (safe_flag) seed_safe else seed_trusted;

    // Use a dynamic import to avoid the cycle that an
    // ``@import("../interp/tcl_interp.zig")`` at file scope would
    // create.  ``eval_script`` runs the seed in the swapped-in
    // child context.
    const tcl_interp = @import("../interp/tcl_interp.zig");
    const save = interp_reg.enter(child);
    defer interp_reg.leave(save);
    _ = tcl_interp.eval_script(@intFromPtr(script.ptr), @intCast(script.len));
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
            // Phase 9: drop every cross-interp variable link whose
            // local or target endpoint is the deleted interp so a
            // dangling link can't outlive its endpoint and route
            // future ``var_resolve`` probes into freed storage.
            const xlinks = @import("../interp/tcl_xlinks.zig");
            xlinks.drop_for_interp(target);
            _ = interp_reg.child_delete(parent, t.name_ptr, t.name_len);
        }
        // Flush the LRU — a parent alias that targets a command in
        // the deleted child would otherwise stay cached.
        procs.lru_invalidate_all();
    }
    return 0;
}

/// ``interp target path alias`` — return the path to the interp
/// hosting ``alias``'s target command, relative to the caller.
/// Mirrors ``GetInterp2`` / ``OPT_TARGET`` in tclInterp.c.
///
/// Algorithm:
///   1. Resolve ``path`` to a source interp (the interp the alias
///      lives in).  Path resolution errors keep the canonical
///      ``could not find interpreter "X"`` wording.
///   2. Look up the alias in the source interp; missing → emit
///      ``alias "FOO" in path "X" not found``.
///   3. Read the alias's stashed ``parent_interp`` (the interp
///      whose ``cmd_table`` holds the target command — zero
///      means "same-interp alias, target lives in the caller's
///      own interp").
///   4. Walk from the target interp up through ``parent`` slots
///      until we reach the caller; collect the simple names in
///      reverse order to build the descendant path.  If we walk
///      past the root without finding the caller, the target
///      isn't ours → raise the ``not my descendant`` error.
pub fn eval_interp_target(words: []const i32) i32 {
    if (words.len != 4) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp target path alias\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const src_interp = resolve_interp_path(words[2]);
    if (src_interp == 0) return 0;
    const alias_name = obj_ensure_string(words[3]);

    // Find the alias in the source interp.  Aliases live in the
    // interp's namespace tree as Commands flagged CMD_ALIAS — walk
    // from the root.
    const src: *interp_reg.Interp = @ptrFromInt(src_interp);
    const alias_cmd = tcl_ns.ns_find_command(src.root_ns, alias_name.ptr, alias_name.len);
    if (!alias_mod.is_alias(alias_cmd)) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const path = obj_ensure_string(words[2]);
        const prefix: []const u8 = "alias \"";
        const infix: []const u8 = "\" in path \"";
        const suffix: []const u8 = "\" not found";
        const total: u32 = @as(u32, @intCast(prefix.len)) +
            alias_name.len +
            @as(u32, @intCast(infix.len)) +
            path.len +
            @as(u32, @intCast(suffix.len));
        const buf = alloc(total);
        const d: [*]u8 = @ptrFromInt(buf);
        var off: u32 = 0;
        for (prefix) |b| {
            d[off] = b;
            off += 1;
        }
        if (alias_name.len > 0) {
            const ap: [*]const u8 = @ptrFromInt(alias_name.ptr);
            for (0..alias_name.len) |k| {
                d[off + k] = ap[k];
            }
            off += alias_name.len;
        }
        for (infix) |b| {
            d[off] = b;
            off += 1;
        }
        if (path.len > 0) {
            const pp: [*]const u8 = @ptrFromInt(path.ptr);
            for (0..path.len) |k| {
                d[off + k] = pp[k];
            }
            off += path.len;
        }
        for (suffix) |b| {
            d[off] = b;
            off += 1;
        }
        const msg = rt.obj_new_string(@bitCast(buf), @bitCast(off));
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

    // Resolve the target interp.  ``parent_interp == 0`` is the
    // single-interp shortcut: the alias's target Command lives in
    // the same interp as the alias itself, i.e. ``src_interp``
    // resolved from ``words[2]``.  Previously this branch fell
    // through to ``interp_current()``, which returned the caller
    // instead of the alias's source interp when ``interp target``
    // was invoked from a sibling / parent (Codex / Copilot review).
    const rec = alias_mod.alias_rec(alias_cmd);
    const target_interp: u32 = if (rec.parent_interp == 0)
        src_interp
    else
        rec.parent_interp;

    // Walk from target up through parent pointers, collecting
    // simple names, until we hit ``my_interp`` (the caller).
    // The collected names are in reverse order; rebuild as a
    // forward list when emitting the result.
    const my_interp = interp_reg.interp_current();
    if (target_interp == my_interp) {
        return obj_new_string(0, 0);
    }

    // ``MAX_PATH_DEPTH`` is a static cap on the descendant chain
    // we walk.  Real Tcl walks unbounded but the WASM runtime's
    // interp tree is shallow in practice; 64 frames is more than
    // tcltest needs and keeps the stack buffer fixed.
    const MAX_PATH_DEPTH: u32 = 64;
    var name_ptrs: [MAX_PATH_DEPTH]u32 = undefined;
    var name_lens: [MAX_PATH_DEPTH]u32 = undefined;
    var depth: u32 = 0;
    var cur = target_interp;
    while (cur != 0 and cur != my_interp and depth < MAX_PATH_DEPTH) : (depth += 1) {
        const ci: *interp_reg.Interp = @ptrFromInt(cur);
        name_ptrs[depth] = ci.name_ptr;
        name_lens[depth] = ci.name_len;
        cur = ci.parent;
    }
    if (cur != my_interp) {
        // Walked past root without finding my_interp — target
        // lives outside the caller's subtree.  Raise the canonical
        // ``not my descendant`` error.
        const catch_mod = @import("../interp/tcl_catch.zig");
        const path = obj_ensure_string(words[2]);
        const prefix: []const u8 = "target interpreter for alias \"";
        const infix: []const u8 = "\" in path \"";
        const suffix: []const u8 = "\" is not my descendant";
        const total: u32 = @as(u32, @intCast(prefix.len)) +
            alias_name.len +
            @as(u32, @intCast(infix.len)) +
            path.len +
            @as(u32, @intCast(suffix.len));
        const buf = alloc(total);
        const d: [*]u8 = @ptrFromInt(buf);
        var off: u32 = 0;
        for (prefix) |b| {
            d[off] = b;
            off += 1;
        }
        if (alias_name.len > 0) {
            const ap: [*]const u8 = @ptrFromInt(alias_name.ptr);
            for (0..alias_name.len) |k| {
                d[off + k] = ap[k];
            }
            off += alias_name.len;
        }
        for (infix) |b| {
            d[off] = b;
            off += 1;
        }
        if (path.len > 0) {
            const pp: [*]const u8 = @ptrFromInt(path.ptr);
            for (0..path.len) |k| {
                d[off + k] = pp[k];
            }
            off += path.len;
        }
        for (suffix) |b| {
            d[off] = b;
            off += 1;
        }
        const msg = rt.obj_new_string(@bitCast(buf), @bitCast(off));
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }

    // Build the forward path: names[depth-1], names[depth-2], ...
    // joined as a Tcl list with single-space separators.
    if (depth == 0) return obj_new_string(0, 0);
    var total_len: u32 = 0;
    var i: u32 = 0;
    while (i < depth) : (i += 1) {
        total_len += name_lens[i] * 2 + 8; // generous quoting
        if (i > 0) total_len += 1;
    }
    const buf = alloc(total_len);
    var off: u32 = 0;
    // Names are in reverse order in name_ptrs (closest-to-target
    // first); emit them in forward order (closest-to-caller first).
    i = depth;
    while (i > 0) {
        i -= 1;
        const is_first = off == 0;
        if (!is_first) {
            const d: [*]u8 = @ptrFromInt(buf + off);
            d[0] = ' ';
            off += 1;
        }
        if (is_first) {
            off = obj_mod.list_elem_quote(buf, off, name_ptrs[i], name_lens[i]);
        } else {
            off = obj_mod.list_elem_quote_nth(buf, off, name_ptrs[i], name_lens[i]);
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
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

/// ``interp marktrusted path`` — clear the ``INTERP_SAFE`` flag on
/// the resolved interp, making a previously-safe child trusted.
/// Mirrors ``MarkTrustedCmd`` / ``OPT_MARKTRUSTED`` in tclInterp.c.
///
/// Permission gate: the *caller* (the interp issuing the command)
/// must be trusted itself.  Calling ``interp marktrusted`` from a
/// safe interpreter raises
/// ``permission denied: safe interpreter cannot mark trusted``.
pub fn eval_interp_marktrusted(words: []const i32) i32 {
    if (words.len != 3) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp marktrusted path\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // Permission gate before path resolution — matches C Tcl,
    // which fires the "permission denied" error even when the path
    // resolves to a non-existent interp.
    if (interp_reg.interp_is_safe(interp_reg.interp_current())) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "permission denied: safe interpreter cannot mark trusted";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const target = resolve_interp_path(words[2]);
    if (target == 0) return 0;
    interp_reg.interp_set_safe(target, false);
    return obj_new_string(0, 0);
}

/// Parse an integer Tcl word into i64.  Accepts the decimal form
/// the runtime's ``try_parse_int`` handles.  Note: this does NOT
/// (yet) accept the full Tcl ``Tcl_GetIntFromObj`` surface — hex
/// (``0x10``), octal (``0o12``), and binary (``0b1010``) literals
/// fall into the ``.not_int`` branch and the caller emits the
/// ``expected integer but got "X"`` diagnostic.  Tracked as a
/// follow-up under the broader ``try_parse_int`` extension
/// (Copilot review on PR #451 — extending this requires touching
/// every other ``try_parse_int`` consumer and is out of scope for
/// the per-interp recursionlimit work).
///
/// Returns:
///
///   ``.ok``         — parsed successfully, ``value`` populated.
///   ``.not_int``    — input wasn't a decimal-integer literal.
///   ``.too_large``  — magnitude overflows i64 (matches Tcl's
///                     ``integer value too large to represent`` path).
const ParseIntResult = struct {
    status: enum { ok, not_int, too_large },
    value: i64,
};

fn parse_int_word(handle: i32) ParseIntResult {
    const s = obj_ensure_string(handle);
    if (s.len == 0) return .{ .status = .not_int, .value = 0 };
    // First try the canonical i64 parser.
    if (obj_mod.try_parse_int(s.ptr, s.len)) |v| {
        return .{ .status = .ok, .value = v };
    }
    // Distinguish "not an integer" from "magnitude overflows".
    // Walk the digits ourselves to detect overflow vs garbage.
    const src: [*]const u8 = @ptrFromInt(s.ptr);
    var i: u32 = 0;
    // Skip leading whitespace
    while (i < s.len) : (i += 1) {
        const c = src[i];
        if (c != ' ' and c != '\t' and c != '\n' and c != '\r' and c != 0x0B and c != 0x0C) break;
    }
    if (i >= s.len) return .{ .status = .not_int, .value = 0 };
    if (src[i] == '+' or src[i] == '-') i += 1;
    if (i >= s.len) return .{ .status = .not_int, .value = 0 };
    var saw_digit = false;
    while (i < s.len) : (i += 1) {
        const c = src[i];
        if (c < '0' or c > '9') break;
        saw_digit = true;
    }
    // Allow trailing whitespace.
    while (i < s.len) : (i += 1) {
        const c = src[i];
        if (c != ' ' and c != '\t' and c != '\n' and c != '\r' and c != 0x0B and c != 0x0C) {
            return .{ .status = .not_int, .value = 0 };
        }
    }
    if (!saw_digit) return .{ .status = .not_int, .value = 0 };
    // Digits-only but try_parse_int returned null — must be an
    // overflow.
    return .{ .status = .too_large, .value = 0 };
}

fn raise_expected_integer(handle: i32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const s = obj_ensure_string(handle);
    const prefix: []const u8 = "expected integer but got \"";
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

/// ``interp recursionlimit path ?newlimit?`` — query or set the
/// per-interp recursion limit.  Mirrors ``RecursionLimitCmd`` /
/// ``OPT_RECLIMIT`` in tclInterp.c.
///
///   - 0 args (``interp recursionlimit``) — wrong-args error.
///   - 1 arg (path) — return the current limit.
///   - 2 args (path, newlimit) — parse newlimit as an integer > 0,
///     install it, return the new value.  Setting from inside a
///     safe interpreter raises
///     ``permission denied: safe interpreters cannot change recursion limit``.
///     Magnitude overflow raises ``integer value too large to represent``.
pub fn eval_interp_recursionlimit(words: []const i32) i32 {
    if (words.len < 3 or words.len > 4) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "wrong # args: should be \"interp recursionlimit path ?newlimit?\"";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // Path-resolution comes before the safe-interp permission check
    // so ``interp recursionlimit nosuchinterp`` raises the
    // ``could not find interpreter`` diagnostic even when called
    // from a safe parent.  Setter form moves the permission check
    // ahead of the integer parse, matching C Tcl.
    const target = resolve_interp_path(words[2]);
    if (target == 0) return 0;
    if (words.len == 3) {
        const cur = interp_reg.interp_recursion_limit(target);
        return obj_new_int(@intCast(cur));
    }
    if (interp_reg.interp_is_safe(interp_reg.interp_current())) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "permission denied: safe interpreters cannot change recursion limit";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    const parsed = parse_int_word(words[3]);
    switch (parsed.status) {
        .not_int => {
            raise_expected_integer(words[3]);
            return 0;
        },
        .too_large => {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "integer value too large to represent";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        },
        .ok => {},
    }
    if (parsed.value < 1) {
        const catch_mod = @import("../interp/tcl_catch.zig");
        const err_text = "recursion limit must be > 0";
        const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
        catch_mod.tcl_cmd_error(msg);
        return 0;
    }
    // Clamp to u32 range — we already rejected magnitudes that
    // overflowed i64; anything above u32 max is silently truncated
    // to u32 max, matching the deep-recursion-impossible-anyway
    // semantics of the WASM runtime.
    const new_limit: u32 = if (parsed.value > @as(i64, std.math.maxInt(u32)))
        std.math.maxInt(u32)
    else
        @intCast(parsed.value);
    interp_reg.interp_set_recursion_limit(target, new_limit);

    // ``interp recursionlimit {} N`` evaluated INSIDE the target
    // interp (interp == childInterp in C Tcl's check) and where
    // the current ``num_levels`` already exceeds the new limit
    // raises ``falling back due to new recursion limit`` — see
    // ``ChildRecursionLimit`` in tclInterp.c.  This lets the
    // running script discover at the boundary that the limit
    // has been lowered below its current nesting (interp-29.3.4
    // / 29.3.5 / 29.3.7c / 29.3.8b / 29.3.10b / 29.3.11b).
    //
    // C Tcl's ``Tcl_EvalObjv`` ``numLevels++``s ahead of running
    // the ``interp recursionlimit`` handler, so the comparison
    // reads ``numLevels > limit``.  Our :fn:`recursion_check_enter`
    // doesn't fire for builtin commands like ``interp ...``, so we
    // adjust by adding 1 here — the +1 represents the recursionlimit
    // dispatch itself that would have ++d in C Tcl's accounting.
    if (target == interp_reg.interp_current()) {
        const ci: *interp_reg.Interp = @ptrFromInt(target);
        if (ci.num_levels + 1 > new_limit) {
            const catch_mod = @import("../interp/tcl_catch.zig");
            const err_text = "falling back due to new recursion limit";
            const msg = rt.obj_new_string_copy(@intFromPtr(err_text.ptr), err_text.len);
            catch_mod.tcl_cmd_error(msg);
            return 0;
        }
    }

    return obj_new_int(@intCast(new_limit));
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
    // Names stored on Command can be either fully qualified
    // (``::a::b::c``) for procs registered via a qualified path
    // or *unqualified* tail names for procs registered into a
    // specific namespace.  ``namespace which`` always returns the
    // FQ form, so promote unqualified names to ``::`` (root) or
    // splice in the home namespace if we can find it cheaply.
    const sp: [*]const u8 = @ptrFromInt(name_ptr);
    if (name_len >= 2 and sp[0] == ':' and sp[1] == ':') {
        return obj_new_string(@bitCast(name_ptr), @bitCast(name_len));
    }
    // Stored name lacks ``::`` prefix.  Synthesise ``::name``.
    const obj_h = @import("../valtypes/tcl_obj.zig");
    const total: u32 = name_len + 2;
    const buf = obj_h.alloc(total);
    if (buf == 0) return obj_new_string(@bitCast(name_ptr), @bitCast(name_len));
    const dst: [*]u8 = @ptrFromInt(buf);
    dst[0] = ':';
    dst[1] = ':';
    for (0..name_len) |k| dst[2 + k] = sp[k];
    return obj_h.obj_new_string_take(buf, total, total);
}

/// Render the FQN of a namespace variable: ``<ns_full>::<name>``
/// with the root-ns ``::`` collapsed so a root-level variable
/// reads as ``::x`` rather than ``::::x``.  Matches
/// :func:`tcl_ns.ns_build_fqn`.
pub fn variable_fqn_obj(ns: u32, simple_ptr: u32, simple_len: u32) i32 {
    const fqn = tcl_ns.ns_build_fqn(ns, simple_ptr, simple_len);
    return obj_new_string(@bitCast(fqn.ptr), @bitCast(fqn.len));
}

/// Raise the canonical ``wrong # args: should be "MSG"`` diagnostic
/// — local helper for :fn:`eval_namespace_which` so it doesn't
/// duplicate the boilerplate eight other namespace subcommands
/// rely on (the central table in :mod:`cmds.namespace` doesn't
/// cover ``namespace which`` because the rules depend on whether
/// the first word starts with ``-``).
fn raise_ns_wrong_args(usage: []const u8) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const obj_helpers = @import("../valtypes/tcl_obj.zig");
    const prefix: []const u8 = "wrong # args: should be \"";
    const suffix: []const u8 = "\"";
    const total: u32 = @intCast(prefix.len + usage.len + suffix.len);
    const buf = obj_helpers.alloc(total);
    if (buf == 0) {
        catch_mod.tcl_cmd_error(obj_helpers.obj_new_string(0, 0));
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    for (usage) |c| {
        dst[off] = c;
        off += 1;
    }
    for (suffix) |c| {
        dst[off] = c;
        off += 1;
    }
    const e = obj_helpers.obj_new_string_take(buf, total, total);
    catch_mod.tcl_cmd_error(e);
}

/// Build the FQN ``<ns>::<simple>`` and probe the flat-key global
/// store.  Returns true when the variable exists in the runtime's
/// global table under the qualified key (the path
/// :func:`tcl_ns.global_set` uses for top-level / namespace-eval
/// writes of qualified names).
fn variable_exists_in_flat_globals(ns: u32, simple_ptr: u32, simple_len: u32) bool {
    const obj_helpers = @import("../valtypes/tcl_obj.zig");
    const fqn = tcl_ns.ns_build_fqn(ns, simple_ptr, simple_len);
    const qname = obj_new_string(@bitCast(fqn.ptr), @bitCast(fqn.len));
    const globals_mod = @import("../interp/tcl_ns.zig");
    const r = globals_mod.global_exists(qname);
    obj_helpers.tcl_obj_release(qname);
    return obj_helpers.obj_get_int(r) != 0;
}

pub fn eval_namespace_which(words: []const i32) i32 {
    // Argument shapes:
    //   namespace which name                         → -command (default)
    //   namespace which -command name
    //   namespace which -variable name
    //
    // Tcl 9 ``NamespaceWhichCmd`` raises ``wrong # args`` when the
    // total argc is out of [3..4] OR when the 3-arg form's leading
    // option doesn't prefix-match ``-command`` / ``-variable``.
    // ``namespace which -command`` (2 args, single ``-command``)
    // treats ``-command`` as the *name* to look up — matches Tcl
    // 9 namespace-34.3 / namespace-old-6.18.
    if (words.len < 3 or words.len > 4) {
        raise_ns_wrong_args("namespace which ?-command? ?-variable? name");
        return obj_new_string(0, 0);
    }

    var which_variable = false;
    var name_idx: u32 = 2;
    if (words.len == 4) {
        const flag = obj_ensure_string(words[2]);
        const fp: [*]const u8 = @ptrFromInt(flag.ptr);
        if (str_eq(fp, flag.len, "-variable")) {
            which_variable = true;
        } else if (!str_eq(fp, flag.len, "-command")) {
            // 3-arg form with unknown option — Tcl 9 raises
            // wrong-args (namespace-34.2 / namespace-old-6.17).
            raise_ns_wrong_args("namespace which ?-command? ?-variable? name");
            return obj_new_string(0, 0);
        }
        name_idx = 3;
    }
    // 2-arg form (words.len == 3): the single argument is the name —
    // even when it starts with ``-`` (``namespace which -command``
    // looks up a command literally named ``-command``).
    const name = obj_ensure_string(words[name_idx]);
    if (name.len == 0) return obj_new_string(0, 0);

    if (which_variable) {
        // Variable resolution: qualified names walk the ns tree to
        // the target ns + simple name, unqualified names check the
        // current ns only (C Tcl doesn't walk ``namespace path``
        // for variables — the path is commands-only).  We also
        // probe the flat-key global store (the runtime's compiled
        // writes for ``set ::A::B::X 99`` land there) so vars
        // created via FQN-prefixed ``set`` are visible too.
        const cxt = tcl_ns.ns_current();
        const r = tcl_ns.ns_resolve_qualified(cxt, name.ptr, name.len);
        if (r.simple_len == 0) return obj_new_string(0, 0);
        if (r.target_ns != 0) {
            const v = tcl_ns.ns_var_find(r.target_ns, r.simple_ptr, r.simple_len);
            if (v != 0) return variable_fqn_obj(r.target_ns, r.simple_ptr, r.simple_len);
            // Flat-key fallback: build the FQN and probe the global
            // store directly.
            if (variable_exists_in_flat_globals(r.target_ns, r.simple_ptr, r.simple_len))
                return variable_fqn_obj(r.target_ns, r.simple_ptr, r.simple_len);
        }
        if (r.alt_ns != 0) {
            const v = tcl_ns.ns_var_find(r.alt_ns, r.simple_ptr, r.simple_len);
            if (v != 0) return variable_fqn_obj(r.alt_ns, r.simple_ptr, r.simple_len);
            if (variable_exists_in_flat_globals(r.alt_ns, r.simple_ptr, r.simple_len))
                return variable_fqn_obj(r.alt_ns, r.simple_ptr, r.simple_len);
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
