// Command: info — introspection into the interpreter state.
//
// Subcommands implemented:
//   info exists varName      — variable exists in the current frame?
//   info commands ?pattern?  — live commands reachable from the
//                              current ns's resolution chain (name
//                              walk + path + root).  Patterns may be
//                              qualified (``::foo::*``) or bare —
//                              both match against simple names.
//                              Hidden-table entries and dead import
//                              redirects are excluded.
//   info procs    ?pattern?  — same walk as ``info commands`` but
//                              filtered to interpreted procs
//                              (``func_idx == 0 && body_obj != 0``).
//                              Matches tclsh's "only interpreted
//                              procs are procs".
//   info body     procName   — body TclObj of an interpreted proc.
//   info args     procName   — params TclObj of an interpreted proc.
//   info default  proc arg var — write the default value of ``arg``
//                              into ``var`` and return 1 if the
//                              parameter has a default, else 0.
//   info complete script     — braces / brackets / quotes all
//                              balanced?  tcltest uses this to
//                              validate user-supplied scripts
//                              before evaluation.
//   info nameofexecutable    — placeholder.
//   info tclversion          — "8.6".
//   info patchlevel          — "8.6.15".
//   info hostname            — "wasm".
//
// All walkers use the shared 16-byte bucket layout
// (``[0..3]`` name_ptr, ``[4..7]`` name_len, ``[8..11]`` hash,
// ``[12..15]`` value = Command*) and read ``Namespace.cmd_table`` /
// ``Namespace.child_table`` inline — no allocation per probed
// bucket.  Result strings are built in one pass with a
// bump-allocated output buffer sized up front.

const obj = @import("tcl_obj.zig");
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const alloc = obj.alloc;
const memcpy = obj.memcpy;

const frames = @import("tcl_frames.zig");
const procs = @import("tcl_procs.zig");
const tcl_ns = @import("tcl_ns.zig");
const tcl_string = @import("tcl_string.zig");

const str_eq = @import("tcl_chars.zig").str_eq;

const BUCKET_SIZE: u32 = 16;

// -- Small exports preserved for the interpreter dispatch -----------------

/// info exists varName — returns 1 if variable is defined, 0 otherwise.
/// Checks current frame locals first, then globals.
pub export fn info_exists(name: i32) i32 {
    return frames.var_exists(name);
}

/// Raise ``"X" isn't a procedure`` with the ``TCL LOOKUP PROCEDURE X``
/// error code path that ``info body`` / ``info args`` /
/// ``info default`` all share when the name doesn't resolve to an
/// interpreted proc.  Matches the Tcl 9 wording verbatim.
fn raise_not_a_procedure(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("tcl_catch.zig");
    const prefix = "\"";
    const suffix = "\" isn't a procedure";
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    d[0] = '"';
    if (name_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| d[prefix.len + k] = sp[k];
    }
    for (suffix, 0..) |b, k| d[prefix.len + name_len + k] = b;
    const msg = obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}

/// Resolve a name to its Command and verify it's an interpreted
/// proc.  On miss (unknown / alias / hidden / compiled proc), raise
/// ``"X" isn't a procedure`` and return 0.  Shared by
/// body / args / default so the error surface stays uniform.
///
/// ``proc_lookup`` transparently unwraps ``CMD_IMPORTED`` to the
/// terminal source Command, so imports that point at a real
/// interpreted proc succeed here.  Aliases don't unwrap (by design
/// — ``CMD_ALIAS`` is preserved for introspection) and fail the
/// filter.  Compiled procs (``func_idx != 0``) have no body TclObj
/// so they fail the ``body == 0`` check, matching Tcl's "no
/// retrievable body" rule.
fn resolve_interpreted_proc(name: i32) u32 {
    const sn = obj_ensure_string(name);
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const cmd: u32 = @bitCast(bucket);
    const flags: u32 = @bitCast(read_i32(cmd + procs.OFF_FLAGS));
    if ((flags & procs.CMD_ALIAS) != 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const func_idx = read_i32(cmd + procs.OFF_FUNC_IDX);
    if (func_idx != 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const body = procs.proc_get_body(bucket);
    if (body == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    return cmd;
}

/// info body procName — returns the body TclObj for an interpreted
/// proc.  Raises ``"X" isn't a procedure`` for anything else
/// (missing, alias, hidden, compiled) and returns an empty TclObj
/// so the error-flag path is the authoritative signal.
pub export fn info_body(name: i32) i32 {
    const cmd = resolve_interpreted_proc(name);
    if (cmd == 0) return obj_new_string(0, 0);
    return procs.proc_get_body(@bitCast(cmd));
}

/// info args procName — returns the params TclObj for an
/// interpreted proc.  Same error shape as ``info body``.
pub export fn info_args(name: i32) i32 {
    const cmd = resolve_interpreted_proc(name);
    if (cmd == 0) return obj_new_string(0, 0);
    return procs.proc_get_params(@bitCast(cmd));
}

/// info complete script — returns 1 if the script has matched
/// braces / brackets / quotes so ``eval`` would accept it, 0
/// otherwise.  This is a structural sanity check used by tcltest
/// and other harnesses to validate user-supplied scripts before
/// evaluating them.
///
/// Brackets inside braced text (``{a [b} c]``) do NOT count —
/// brace-quoted words treat ``[`` as a literal character, not a
/// command-substitution marker.  So a script like ``"{[}"`` is
/// structurally complete: the outer ``[`` / ``]`` are consumed by
/// the braced word, which is itself balanced.  Only brackets
/// outside brace depth 0 affect completeness.  Similarly quotes
/// are literal inside braces.
pub fn info_complete(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return obj_new_int(1);
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    var brace: i32 = 0;
    var bracket: i32 = 0;
    var in_quote: bool = false;
    var i: u32 = 0;
    while (i < s.len) : (i += 1) {
        const c = sp[i];
        if (c == '\\' and i + 1 < s.len) {
            i += 1;
            continue;
        }
        if (brace > 0) {
            switch (c) {
                '{' => brace += 1,
                '}' => brace -= 1,
                else => {},
            }
            continue;
        }
        if (in_quote) {
            if (c == '"') in_quote = false;
            continue;
        }
        switch (c) {
            '{' => brace += 1,
            '}' => brace -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            '"' => in_quote = true,
            else => {},
        }
        if (brace < 0 or bracket < 0) return obj_new_int(0);
    }
    if (brace == 0 and bracket == 0 and !in_quote) return obj_new_int(1);
    return obj_new_int(0);
}

pub fn info_nameofexecutable() i32 {
    return obj.obj_new_string_copy(@intFromPtr("tcl_runtime.wasm".ptr), 16);
}

pub fn info_tclversion() i32 {
    return obj.obj_new_string_copy(@intFromPtr("8.6".ptr), 3);
}

pub fn info_patchlevel() i32 {
    return obj.obj_new_string_copy(@intFromPtr("8.6.15".ptr), 6);
}

pub fn info_hostname() i32 {
    return obj.obj_new_string_copy(@intFromPtr("wasm".ptr), 4);
}

// -- ``info commands`` / ``info procs`` walker ----------------------------

/// Kinds of entry the cmd-table walker emits.  ``commands`` yields
/// every live Command (including aliases and import redirects);
/// ``procs`` narrows to interpreted procs only.
const WalkKind = enum { commands, procs };

/// Context carried through both the sizing + filling passes.  Using
/// two passes keeps the output buffer tightly sized — the bump
/// allocator can't realloc, so we can't grow in place.
const CmdWalkCtx = struct {
    kind: WalkKind,
    /// Non-null when a pattern was supplied.  Matched against the
    /// simple (unqualified) name of each candidate.
    pat_ptr: u32,
    pat_len: u32,
    has_pattern: bool,
    /// Cumulative length of the fully-qualified names we'll emit,
    /// plus one separator byte per gap.  Populated during pass 1.
    total: u32,
    /// Running count of emitted entries.  Used for the space-before-
    /// each-after-first separator pattern.
    count: u32,
    /// Only populated during pass 2.
    buf: u32,
    off: u32,
};

/// Predicate: does this Command match the current filter kind and
/// pattern?  Called once per live bucket in both the sizing and
/// filling passes; encapsulates the per-bucket filtering logic so
/// the two walkers stay in lockstep.
fn entry_matches(ctx: *const CmdWalkCtx, name_ptr: u32, name_len: u32, cmd: u32) bool {
    if (cmd == 0) return false;
    const flags: u32 = @bitCast(read_i32(cmd + procs.OFF_FLAGS));
    // Dead import redirect (``namespace forget``).  Live imports
    // have ``ImportedCmdData.real_cmd != 0``; forgotten ones zero
    // that slot.  Skip both kinds so stale entries disappear from
    // ``info commands``.
    if ((flags & procs.CMD_IMPORTED) != 0) {
        const desc: u32 = @bitCast(read_i32(cmd + procs.OFF_PARAMS_OBJ));
        if (desc == 0) return false;
        const real: u32 = @bitCast(read_i32(desc));
        if (real == 0) return false;
    }
    switch (ctx.kind) {
        .commands => {},
        .procs => {
            // ``info procs`` only reports interpreted procs —
            // compiled, alias, and import redirects don't qualify.
            if ((flags & procs.CMD_ALIAS) != 0) return false;
            if ((flags & procs.CMD_IMPORTED) != 0) return false;
            const func_idx = read_i32(cmd + procs.OFF_FUNC_IDX);
            if (func_idx != 0) return false;
            const body = read_i32(cmd + 16); // OFF_BODY_OBJ
            if (body == 0) return false;
        },
    }
    if (ctx.has_pattern) {
        if (!tcl_string.glob_match(ctx.pat_ptr, ctx.pat_len, name_ptr, name_len)) {
            return false;
        }
    }
    return true;
}

/// Compute the length of a Command's fully-qualified name — we read
/// it straight out of the Command's stored name slot.  ``rename`` /
/// ``expose`` keep this slot in sync with the live identity.
fn cmd_stored_name(cmd: u32) struct { ptr: u32, len: u32 } {
    const p: u32 = @bitCast(read_i32(cmd));
    const l: u32 = @bitCast(read_i32(cmd + 4));
    return .{ .ptr = p, .len = l };
}

/// Probe one namespace's ``cmd_table`` for matching entries.  If
/// ``ctx.buf == 0`` we're in the sizing pass and just increment
/// ``ctx.total`` / ``ctx.count``; otherwise we append each FQN into
/// ``ctx.buf`` separated by single spaces.
fn scan_ns_cmd_table(ns_addr: u32, ctx: *CmdWalkCtx) void {
    const ns: *const tcl_ns.Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return;
    var i: u32 = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        const cmd: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (!entry_matches(ctx, name_ptr, name_len, cmd)) continue;
        const fqn = cmd_stored_name(cmd);
        if (fqn.len == 0) continue;
        emit_name(ctx, fqn.ptr, fqn.len);
    }
}

/// Emit one FQN — sizing pass if ``ctx.buf == 0``, filling pass
/// otherwise.  The separator is a single space between entries.
fn emit_name(ctx: *CmdWalkCtx, name_ptr: u32, name_len: u32) void {
    if (ctx.buf == 0) {
        if (ctx.count > 0) ctx.total += 1;
        ctx.total += name_len;
        ctx.count += 1;
        return;
    }
    if (ctx.count > 0) {
        const d: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
        d[0] = ' ';
        ctx.off += 1;
    }
    memcpy(ctx.buf + ctx.off, name_ptr, name_len);
    ctx.off += name_len;
    ctx.count += 1;
}

/// Walk the current ns's resolution chain (context + namespace path
/// + root) and emit every matching Command.  Mirrors
/// ``InfoCommandsCmd`` in ``tclCmdIL.c`` for the unqualified case.
///
/// De-duplication: we visit root last, and the context-ns probe
/// already covers ``root == current_ns`` directly, so a root-level
/// command reachable from the root ns appears once rather than
/// twice.  Path entries pointing back at the context are skipped
/// (mirrors ``ns_find_command``).
fn walk_unqualified_path(ctx: *CmdWalkCtx) void {
    const root = tcl_ns.ns_root();
    const start: u32 = tcl_ns.ns_current();

    scan_ns_cmd_table(start, ctx);

    const plen = tcl_ns.ns_path_len(start);
    var pi: u32 = 0;
    while (pi < plen) : (pi += 1) {
        const e = tcl_ns.ns_path_entry(start, pi);
        if (e.target_ns == 0 or e.target_ns == start) continue;
        scan_ns_cmd_table(e.target_ns, ctx);
    }

    if (start != root) scan_ns_cmd_table(root, ctx);
}

/// Walk only the target namespace's ``cmd_table`` — used when the
/// caller supplied a qualified pattern (``::foo::*``).  The
/// pattern's trailing simple component is matched against each
/// name; the leading namespace component has already been stripped
/// by the qualified-pattern parser.
fn walk_qualified(target_ns: u32, ctx: *CmdWalkCtx) void {
    if (target_ns == 0) return;
    scan_ns_cmd_table(target_ns, ctx);
}

/// Produce a TclObj list of matching FQNs, space-separated.
/// ``pattern`` may be empty (no filter), unqualified (match
/// against simple names in the current-ns walk), or qualified
/// (split into target-ns + simple-pattern via
/// ``ns_resolve_qualified``).
fn run_cmd_walk(kind: WalkKind, pattern: i32) i32 {
    var ctx: CmdWalkCtx = .{
        .kind = kind,
        .pat_ptr = 0,
        .pat_len = 0,
        .has_pattern = false,
        .total = 0,
        .count = 0,
        .buf = 0,
        .off = 0,
    };

    var qualified_target: u32 = 0;
    if (pattern != 0) {
        const p = obj_ensure_string(pattern);
        if (p.len > 0) {
            ctx.has_pattern = true;
            ctx.pat_ptr = p.ptr;
            ctx.pat_len = p.len;
            // Detect qualified pattern: any ``::`` substring.
            var has_colons = false;
            const src: [*]const u8 = @ptrFromInt(p.ptr);
            if (p.len >= 2) {
                var i: u32 = 0;
                while (i + 1 < p.len) : (i += 1) {
                    if (src[i] == ':' and src[i + 1] == ':') {
                        has_colons = true;
                        break;
                    }
                }
            }
            if (has_colons) {
                const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), p.ptr, p.len);
                qualified_target = r.target_ns;
                ctx.pat_ptr = r.simple_ptr;
                ctx.pat_len = r.simple_len;
                if (ctx.pat_len == 0) {
                    // Pattern was ``::`` alone or trailing ``::`` —
                    // matches nothing.
                    return obj_new_string(0, 0);
                }
            }
        }
    }

    // Pass 1: size.
    if (qualified_target != 0) {
        walk_qualified(qualified_target, &ctx);
    } else {
        walk_unqualified_path(&ctx);
    }
    if (ctx.total == 0) return obj_new_string(0, 0);

    // Pass 2: fill.
    ctx.buf = alloc(ctx.total);
    ctx.off = 0;
    ctx.count = 0;
    if (qualified_target != 0) {
        walk_qualified(qualified_target, &ctx);
    } else {
        walk_unqualified_path(&ctx);
    }
    return obj_new_string(@bitCast(ctx.buf), @bitCast(ctx.off));
}

/// info commands ?pattern? — public entry.  ``pattern == 0`` means
/// "no pattern supplied"; a non-zero TclObj is treated as the
/// filter.
pub fn info_commands(pattern: i32) i32 {
    return run_cmd_walk(.commands, pattern);
}

/// info procs ?pattern? — same walk, interpreted-procs-only filter.
pub fn info_procs(pattern: i32) i32 {
    return run_cmd_walk(.procs, pattern);
}

// -- ``info default`` ------------------------------------------------------

/// Raise ``procedure "X" doesn't have an argument "Y"`` — the Tcl 9
/// error string for ``info default`` when the arg name isn't in the
/// proc's params list.
fn raise_missing_argument(proc_ptr: u32, proc_len: u32, arg_ptr: u32, arg_len: u32) void {
    const catch_mod = @import("tcl_catch.zig");
    const prefix = "procedure \"";
    const middle = "\" doesn't have an argument \"";
    const suffix = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len)) + proc_len +
        @as(u32, @intCast(middle.len)) + arg_len +
        @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix, 0..) |b, k| d[k] = b;
    off = @intCast(prefix.len);
    if (proc_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(proc_ptr);
        for (0..proc_len) |k| d[off + k] = sp[k];
    }
    off += proc_len;
    for (middle, 0..) |b, k| d[off + k] = b;
    off += @as(u32, @intCast(middle.len));
    if (arg_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(arg_ptr);
        for (0..arg_len) |k| d[off + k] = sp[k];
    }
    off += arg_len;
    for (suffix, 0..) |b, k| d[off + k] = b;
    const msg = obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}

/// info default proc arg varName — write the default value of
/// parameter ``arg`` in proc ``proc`` into the caller's variable
/// ``varName`` and return 1 when a default is present.  When the
/// arg has no default, set ``varName`` to the empty string and
/// return 0.  When ``arg`` isn't in the proc's params list, raise
/// ``procedure "X" doesn't have an argument "Y"`` (tclsh parity).
/// Non-interpreted-proc resolution failures are handled in
/// ``resolve_interpreted_proc``.
pub export fn info_default(proc_name: i32, arg_name: i32, var_name: i32) i32 {
    const cmd = resolve_interpreted_proc(proc_name);
    if (cmd == 0) return obj_new_int(0);
    const params_obj = procs.proc_get_params(@bitCast(cmd));
    const arg_s = obj_ensure_string(arg_name);
    const proc_s = obj_ensure_string(proc_name);
    if (params_obj == 0) {
        raise_missing_argument(proc_s.ptr, proc_s.len, arg_s.ptr, arg_s.len);
        return obj_new_int(0);
    }
    const params_s = obj_ensure_string(params_obj);

    const count = obj.list_count_elements(params_s.ptr, params_s.len);
    var i: i64 = 0;
    while (i < count) : (i += 1) {
        const elt = obj.list_element_at(params_s.ptr, params_s.len, i);
        // Is this a two-element ``{name default}`` pair?  Split the
        // element into its own sub-elements and compare.
        const sub_count = obj.list_count_elements(elt.start, elt.len);
        if (sub_count == 0) continue;
        const name_sub = obj.list_element_at(elt.start, elt.len, 0);
        if (name_sub.len != arg_s.len) continue;
        if (name_sub.len == 0) continue;
        const ap: [*]const u8 = @ptrFromInt(arg_s.ptr);
        const np: [*]const u8 = @ptrFromInt(name_sub.start);
        var match = true;
        var k: u32 = 0;
        while (k < arg_s.len) : (k += 1) {
            if (ap[k] != np[k]) {
                match = false;
                break;
            }
        }
        if (!match) continue;
        // Found the parameter.  If it has a default, pass that
        // through ``var_name``; otherwise set ``var_name`` to empty
        // (Tcl 9 InfoDefaultCmd sets to a fresh empty TclObj so the
        // caller's variable is always materialised, unlike the
        // pre-wave behaviour of leaving it unset).
        if (sub_count < 2) {
            _ = frames.var_set(var_name, obj_new_string(0, 0));
            return obj_new_int(0);
        }
        const default_sub = obj.list_element_at(elt.start, elt.len, 1);
        const default_obj = obj_new_string(@bitCast(default_sub.start), @bitCast(default_sub.len));
        _ = frames.var_set(var_name, default_obj);
        return obj_new_int(1);
    }
    // Arg isn't in the proc's params — match tclsh's error shape.
    raise_missing_argument(proc_s.ptr, proc_s.len, arg_s.ptr, arg_s.len);
    return obj_new_int(0);
}

// -- Dispatch --------------------------------------------------------------

/// Dispatch for 'info' command — one-word / two-word cases.  The
/// multi-arg cases (``info default``) are routed via a separate
/// entry; the interpreter's ``info`` handler picks whichever
/// dispatcher matches the arity.
pub export fn info_dispatch(subcmd: i32, arg: i32) i32 {
    const sub = obj_ensure_string(subcmd);
    if (sub.len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);

    if (str_eq(sp, sub.len, "exists")) return info_exists(arg);
    if (str_eq(sp, sub.len, "body")) return info_body(arg);
    if (str_eq(sp, sub.len, "args")) return info_args(arg);
    if (str_eq(sp, sub.len, "complete")) return info_complete(arg);
    if (str_eq(sp, sub.len, "nameofexecutable")) return info_nameofexecutable();
    if (str_eq(sp, sub.len, "tclversion")) return info_tclversion();
    if (str_eq(sp, sub.len, "patchlevel")) return info_patchlevel();
    if (str_eq(sp, sub.len, "hostname")) return info_hostname();
    if (str_eq(sp, sub.len, "commands")) return info_commands(arg);
    if (str_eq(sp, sub.len, "procs")) return info_procs(arg);
    return obj_new_string(0, 0);
}
