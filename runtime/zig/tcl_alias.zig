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

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_string = obj.obj_new_string;

const tcl_ns = @import("tcl_ns.zig");
const tcl_procs = @import("tcl_procs.zig");

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
};

comptime {
    if (@sizeOf(AliasRec) != 16) @compileError("AliasRec layout drift");
}

/// Allocate and populate a fresh alias redirect Command.  Returns
/// the Command address.  Caller is responsible for inserting the
/// Command into its destination ns's ``cmd_table`` (via
/// ``ns_cmd_put``).  The target name bytes + prefix arg handles
/// are both heap-copied so the caller's input can be released.
pub fn alias_alloc(
    new_simple_ptr: u32,
    new_simple_len: u32,
    target_name_ptr: u32,
    target_name_len: u32,
    n_prefix: u32,
    prefix_args_addr: u32,
) u32 {
    const cmd = alloc(tcl_procs.COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(cmd);
    @memset(slice[0..tcl_procs.COMMAND_SIZE], 0);

    // Heap-copy the redirect's own simple name (for
    // ``proc_get_name_ptr`` / ``info commands`` consumers).
    const nbuf = alloc(new_simple_len);
    if (new_simple_len > 0) memcpy(nbuf, new_simple_ptr, new_simple_len);
    write_i32(cmd, @bitCast(nbuf));
    write_i32(cmd + 4, @bitCast(new_simple_len));

    // Stamp the flag.
    write_i32(cmd + tcl_procs.OFF_FLAGS, @bitCast(CMD_ALIAS));

    // Heap-copy the target name bytes.
    const tbuf = alloc(target_name_len);
    if (target_name_len > 0) memcpy(tbuf, target_name_ptr, target_name_len);

    // Heap-copy the prefix-args array.  Each element is a u32
    // TclObj handle.  We use the bump allocator + write_i32 to
    // avoid the ``[*]u32`` alignment cast Zig would otherwise
    // require for a pointer into bump memory.
    var pbuf: u32 = 0;
    if (n_prefix > 0 and prefix_args_addr != 0) {
        pbuf = alloc(n_prefix * 4);
        var i: u32 = 0;
        while (i < n_prefix) : (i += 1) {
            write_i32(pbuf + i * 4, read_i32(prefix_args_addr + i * 4));
        }
    }

    const rec = alloc(@sizeOf(AliasRec));
    const r: *AliasRec = @ptrFromInt(rec);
    r.target_name_ptr = tbuf;
    r.target_name_len = target_name_len;
    r.n_prefix = n_prefix;
    r.prefix_args_addr = pbuf;
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
}

/// Produce the query form string for ``interp alias {} newName``:
/// the target FQN + each prefix arg as a space-separated list
/// (Tcl-list-escaped by the caller if needed).  Returns a TclObj
/// handle.
///
/// The simple concatenation is fine for tcltest's usage — the
/// prefix args in practice are single-word options like ``-setup``
/// or literal keywords.  When a prefix arg contains whitespace,
/// the caller should route through the full Tcl list-quoting
/// helper; we leave that to the ``interp alias`` built-in.
pub fn alias_describe(cmd: u32) i32 {
    if (!is_alias(cmd)) return obj_new_string(0, 0);
    const r = alias_rec(cmd);
    if (r.target_name_len == 0) return obj_new_string(0, 0);

    // Compute the total byte length: target + (" " + arg.len) for
    // each prefix arg.  Over-allocates by one when n_prefix == 0.
    var total: u32 = r.target_name_len;
    var i: u32 = 0;
    while (i < r.n_prefix) : (i += 1) {
        const h: i32 = read_i32(r.prefix_args_addr + i * 4);
        const s = obj_ensure_string(h);
        total += 1 + s.len;
    }
    const buf = alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    const tp: [*]const u8 = @ptrFromInt(r.target_name_ptr);
    for (0..r.target_name_len) |k| dst[k] = tp[k];
    var off: u32 = r.target_name_len;
    i = 0;
    while (i < r.n_prefix) : (i += 1) {
        dst[off] = ' ';
        off += 1;
        const h: i32 = read_i32(r.prefix_args_addr + i * 4);
        const s = obj_ensure_string(h);
        if (s.len > 0) {
            const sp: [*]const u8 = @ptrFromInt(s.ptr);
            for (0..s.len) |k| dst[off + k] = sp[k];
            off += s.len;
        }
    }
    return obj_new_string(@bitCast(buf), @bitCast(off));
}
