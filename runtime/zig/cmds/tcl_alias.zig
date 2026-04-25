// Single-interp ``interp alias`` support.
//
// Mirror of Tcl 9's ``AliasCreate`` / ``AliasDelete`` in
// ``tclInterp.c``, trimmed to the single-interp subset ("slave"
// interps are out of scope until the runtime grows child
// interpreters).  An alias is a Command whose dispatch prepends a
// frozen argv prefix and resolves the target command by name at
// call time.  Unlike ``namespace import``, the target is resolved
// by *string* on every dispatch — the alias tracks renames /
// deletions of its target automatically (matching the C semantics).
//
// Dispatch shape:
//
//     alias newName target ?arg1 arg2 …?
//
//     newName arg3 arg4
//       ⇒ target arg1 arg2 arg3 arg4
//
// Layout:
//
//     AliasRec (bump-allocated per alias, 16 bytes):
//       [ 0.. 3] target_name_ptr  : u32  (FQN of the target command)
//       [ 4.. 7] target_name_len  : u32
//       [ 8..11] n_prefix         : u32
//       [12..15] prefix_args_addr : u32  (packed array of TclObj* —
//                                         each slot is an i32 TclObj
//                                         handle, width 4 bytes)
//
// The alias redirect Command is a regular 40-byte ``Command``:
//
//     flags = CMD_ALIAS (0x100)
//     params_obj = *AliasRec
//     body_obj = 0
//     n_params = 0
//     func_idx = 0
//     import_ref_head = 0
//
// ``proc_lookup`` consumes the Command like any other; the
// dispatch site (``eval_proc_call_bucket`` in ``tcl_interp.zig``)
// recognises ``CMD_ALIAS`` and trampolines through
// :func:`dispatch_alias`.  Import unwrap does NOT follow aliases —
// we want the redirect identity preserved for queries like
// ``interp alias {} foo``.

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;

const tcl_ns = @import("../interp/tcl_ns.zig");
const tcl_procs = @import("../interp/tcl_procs.zig");

/// Synthetic flag bit stamped onto alias redirect Commands so the
/// dispatcher recognises them.  Picked from the unused-by-C
/// ``0x100`` slot (C uses 0x1, 0x2, 0x4, 0x10 for cmd flags and
/// 0x80 is already our ``CMD_IMPORTED``).  Duplicated on
/// ``tcl_procs.CMD_ALIAS`` so proc dispatch doesn't need to import
/// this module.
pub const CMD_ALIAS: u32 = 0x100;

pub const AliasRec = extern struct {
    target_name_ptr: u32,
    target_name_len: u32,
    n_prefix: u32,
    prefix_args_addr: u32,
    /// Cross-interp alias source: the parent ``Interp*`` whose
    /// ``cmd_table`` holds the target command.  Zero for same-
    /// interp aliases (matches C Tcl where the parent is implicit);
    /// non-zero triggers a ``tcl_interp_registry.enter`` / ``leave``
    /// pair around ``proc_lookup`` in ``dispatch_alias``.
    ///
    /// Used to live in the Command's ``OFF_IMPORT_REF_HEAD`` slot,
    /// which was unsafe — that slot is shared with
    /// ``link_import_ref``'s back-reference list, so a later
    /// ``namespace import`` of the alias would overwrite the
    /// stashed Interp*.  Keeping it inside ``AliasRec`` keeps the
    /// Command's import-machinery slots intact.
    parent_interp: u32,
};

comptime {
    if (@sizeOf(AliasRec) != 20) @compileError("AliasRec layout drift");
}

/// Allocate and populate a fresh alias redirect Command.  Returns
/// the Command address.  Caller is responsible for inserting the
/// Command into ``dest_ns``'s ``cmd_table`` (via ``ns_cmd_put``).
/// The target name bytes + prefix arg handles are heap-copied so
/// the caller's input can be released.
///
/// The Command's ``name_ptr`` / ``name_len`` header is stamped
/// with the fully-qualified alias name (``<dest_ns_full>::<simple>``)
/// so introspection callers (``proc_get_name_ptr``,
/// ``info commands``, the host-bridge dispatcher) see the same
/// FQN shape they'd get from an interpreted ``proc`` definition.
/// ``dest_ns`` is the destination namespace the caller will insert
/// the Command into.  ``parent_interp`` is the ``Interp*`` whose
/// ``cmd_table`` holds the target command (zero for same-interp
/// aliases where dispatch stays in the current interp).
pub fn alias_alloc(
    dest_ns: u32,
    new_simple_ptr: u32,
    new_simple_len: u32,
    target_name_ptr: u32,
    target_name_len: u32,
    n_prefix: u32,
    prefix_args_addr: u32,
    parent_interp: u32,
) u32 {
    const cmd = alloc(tcl_procs.COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(cmd);
    @memset(slice[0..tcl_procs.COMMAND_SIZE], 0);

    // Stamp the Command's fully-qualified name — matches the
    // shape ``proc_register`` produces for interpreted procs
    // (``::ns::name``) so introspection paths stay uniform.
    const fqn = tcl_ns.ns_build_fqn(dest_ns, new_simple_ptr, new_simple_len);
    write_i32(cmd, @bitCast(fqn.ptr));
    write_i32(cmd + 4, @bitCast(fqn.len));

    // Stamp the flag.
    write_i32(cmd + tcl_procs.OFF_FLAGS, @bitCast(CMD_ALIAS));

    // Heap-copy the target name bytes.
    const tbuf = alloc(target_name_len);
    if (target_name_len > 0) memcpy(tbuf, target_name_ptr, target_name_len);

    // Heap-copy the prefix-args array.  Each element is a u32
    // TclObj handle.  MM-B.6: retain each handle so the alias's
    // prefix array survives the parser-side release of the source
    // words (the dispatch path that called ``interp alias`` will
    // release its words[] after dispatch returns).
    var pbuf: u32 = 0;
    if (n_prefix > 0 and prefix_args_addr != 0) {
        pbuf = alloc(n_prefix * 4);
        var i: u32 = 0;
        while (i < n_prefix) : (i += 1) {
            const handle = read_i32(prefix_args_addr + i * 4);
            write_i32(pbuf + i * 4, handle);
            if (handle != 0) {
                const objm = @import("../valtypes/tcl_obj.zig");
                objm.tcl_obj_retain(handle);
            }
        }
    }

    const rec = alloc(@sizeOf(AliasRec));
    const r: *AliasRec = @ptrFromInt(rec);
    r.target_name_ptr = tbuf;
    r.target_name_len = target_name_len;
    r.n_prefix = n_prefix;
    r.prefix_args_addr = pbuf;
    r.parent_interp = parent_interp;
    write_i32(cmd + tcl_procs.OFF_PARAMS_OBJ, @bitCast(rec));

    return cmd;
}

/// Read the ``AliasRec`` out of an alias redirect Command.  Callers
/// should verify ``flags & CMD_ALIAS != 0`` first; mis-calling
/// returns a pointer to whatever lives in the ``params_obj`` slot.
pub fn alias_rec(cmd: u32) *AliasRec {
    const rec_addr = read_i32(cmd + tcl_procs.OFF_PARAMS_OBJ);
    return @ptrFromInt(@as(u32, @bitCast(rec_addr)));
}

/// Predicate: is ``cmd`` an alias redirect?  ``cmd == 0`` returns
/// false.
pub fn is_alias(cmd: u32) bool {
    if (cmd == 0) return false;
    const flags: u32 = @bitCast(read_i32(cmd + tcl_procs.OFF_FLAGS));
    return (flags & CMD_ALIAS) != 0;
}

/// Deactivate an alias redirect — zero its ``AliasRec`` target-name
/// length so later dispatches surface "target unreachable" rather
/// than jumping into garbage.  The Command's ``cmd_table`` entry is
/// the caller's responsibility (typically via ``ns_cmd_clear``).
pub fn alias_clear(cmd: u32) void {
    if (!is_alias(cmd)) return;
    const r = alias_rec(cmd);
    r.target_name_len = 0;
    r.target_name_ptr = 0;
    r.n_prefix = 0;
    r.prefix_args_addr = 0;
    r.parent_interp = 0;
}

/// Produce the query form for ``interp alias {} newName``: the
/// target FQN plus each prefix arg, rendered as a valid Tcl list
/// via :func:`tcl_obj.list_elem_quote` so round-tripping
/// ``[interp alias {} name]`` through ``lindex`` / ``foreach``
/// recovers the exact original prefix vector.  A prefix arg like
/// ``{hello world}`` renders as ``{hello world}``, a ``$foo``
/// prefix renders as ``{$foo}``, and an empty prefix as ``{}``.
/// Returns a TclObj handle.
pub fn alias_describe(cmd: u32) i32 {
    if (!is_alias(cmd)) return obj_new_string(0, 0);
    const r = alias_rec(cmd);
    if (r.target_name_len == 0) return obj_new_string(0, 0);

    // Generous over-allocation: each source byte can expand to two
    // output bytes in the worst ``\\<byte>`` case, plus the bracing
    // overhead per element.  ``+ 8`` per element covers the worst-
    // case ``{}`` wrapping.  The list-quote helper writes fewer
    // bytes than this budget; unused slack stays in bump memory.
    var total: u32 = r.target_name_len * 2 + 8;
    var i: u32 = 0;
    while (i < r.n_prefix) : (i += 1) {
        const h: i32 = read_i32(r.prefix_args_addr + i * 4);
        const s = obj_ensure_string(h);
        total += 1 + s.len * 2 + 8; // leading space + quoted element
    }
    const buf = alloc(total);

    // Element 0: the target name.  ``list_elem_quote`` treats it as
    // the first list element so a leading ``#`` is braced — matches
    // C Tcl's ``Tcl_Merge``.
    var off: u32 = obj.list_elem_quote(buf, 0, r.target_name_ptr, r.target_name_len);

    // Subsequent elements: each prefix arg, space-separated, with
    // ``list_elem_quote_nth`` so a leading ``#`` on a non-first
    // element stays unbraced (matches ``UpdateStringOfList``).
    i = 0;
    while (i < r.n_prefix) : (i += 1) {
        const d: [*]u8 = @ptrFromInt(buf + off);
        d[0] = ' ';
        off += 1;
        const h: i32 = read_i32(r.prefix_args_addr + i * 4);
        const s = obj_ensure_string(h);
        off = obj.list_elem_quote_nth(buf, off, s.ptr, s.len);
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}
