// Interpreter registry — one ``Interp`` per interpreter, keyed by
// simple name inside its parent.
//
// Ships the minimum viable primitives for child interpreters
// (``interp create`` / ``eval`` / ``delete`` / ``exists`` /
// ``slaves``), plus the per-interp ``hidden_cmd_table`` slot that
// used to live as a module-global in ``tcl_ns.zig``.
//
// Reference C Tcl sources:
//
// * ``tmp/tcl9.0.3/generic/tclBasic.c`` — ``Tcl_CreateInterp`` /
//   ``DeleteInterp`` (per-interp global-ns, hidden-table, call-frame
//   stack).
// * ``tmp/tcl9.0.3/generic/tclInterp.c`` — ``ChildCreate`` /
//   ``ChildEval`` / ``InterpObjCmd`` dispatch.
//
// Storage model (mirrors ``tmp/tcl9.0.3/generic/tclInterp.c``
// ``Parent.childTable`` + ``Child`` struct, trimmed to what the
// WASM runtime consumes):
//
//   Interp (per interp, bump-allocated):
//     root_ns          — this interp's global (::) Namespace*
//     hidden_cmd_table — this interp's hidden commands
//     parent           — parent Interp* (0 for the root interp)
//     name_*           — simple name in parent's children table
//     children         — child-interp registry (name -> Interp*)
//     flags            — INTERP_SAFE bit, etc.
//
// Swap semantics:
//
// ``interp eval child script`` saves ``tcl_ns.root_addr``,
// ``tcl_ns.current_ns``, and ``current_interp``, swaps them to the
// child's slots, dispatches the script, and restores on the way
// out.  The shared ``tcl_ns.root_addr`` avoids a circular import
// cycle (ns ↔ registry); swapping it keeps every ``tcl_ns.ns_root()``
// caller in the rest of the runtime seeing the right root without
// per-call lookups.

const obj = @import("../valtypes/tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("../valtypes/hash_table.zig");
const tcl_ns = @import("tcl_ns.zig");

/// ``-safe`` marker.  Matches C Tcl's ``INTERP_SAFE`` flag semantics
/// loosely — we record the bit but don't gate file/exec/package/env
/// access because the WASM runtime doesn't expose those anyway.  See
/// ``docs/design/runtime/child-interp.md`` §2 for the deferred-gating
/// discussion.
pub const INTERP_SAFE: u32 = 0x1;

/// Set on an ``Interp`` once ``interp delete path`` has torn it down.
/// Dispatch paths that might still hold the stale ``Interp*`` (cross-
/// interp alias redirect Commands whose ``OFF_IMPORT_REF_HEAD`` slot
/// points here, or pending ``enter`` frames unwinding) check this bit
/// and surface a clean "unknown command" diagnostic instead of
/// walking a zeroed ``root_ns``.  Set by :func:`child_delete_recursive`
/// as part of the cascade.
pub const INTERP_DELETED: u32 = 0x2;

/// Bucket size shared across ``children`` and ``hidden_cmd_table``.
/// Keeps ``Table(16)`` monomorphised once; matches the
/// ``NS_BUCKET_SIZE`` used throughout the namespace tree so the
/// ``OFF_HANDLE`` convention (value at offset 12) stays uniform.
pub const BUCKET_SIZE: u32 = 16;

/// Initial sub-table capacity.  Most interps have zero to a handful
/// of children and a small number of hidden commands; 16 buckets
/// keeps the load factor low with minimal waste.
pub const INITIAL_CAP: u32 = 16;

const ChildTable = ht.Table(BUCKET_SIZE);
const HiddenTable = ht.Table(BUCKET_SIZE);

/// Per-interp state.  ``extern struct`` layout for the same reason
/// ``tcl_ns.Namespace`` is extern: we hand bare ``u32`` addresses
/// across the WASM ABI and need guaranteed field offsets.
pub const Interp = extern struct {
    /// This interp's root (global) ``Namespace`` address.  For the
    /// root interp this equals ``tcl_ns.root_addr`` (set when
    /// ``interp_root()`` adopts the lazily-allocated root ns); for
    /// child interps, a fresh ``Namespace`` allocated via
    /// ``tcl_ns.ns_alloc_root``.
    root_ns: u32,

    /// Interpreter-wide hidden-commands table.  C Tcl keeps one
    /// ``hiddenCmdTable`` per interp (``Interp.hiddenCmdTablePtr``)
    /// and cross-interp ``hide`` / ``expose`` primitives route
    /// through the resolved target interp's slot.  Pre-child-interp
    /// this lived as a module-global in ``tcl_ns.zig``.
    hidden_cmd_table: HiddenTable,

    /// Parent interp.  Zero only for the root interp.  Child creation
    /// walks the chain in reverse to build the full path for
    /// introspection helpers (not shipped this wave).
    parent: u32,

    /// Simple name in the parent's ``children`` table.  Zero-len for
    /// the root interp.
    name_ptr: u32,
    name_len: u32,

    /// Children keyed by simple name -> ``Interp*``.  Lazily
    /// initialised: ``buf == 0`` means "no children yet".
    children: ChildTable,

    /// ``INTERP_SAFE | ...``.  Safe interps currently behave
    /// identically to unsafe ones — see the top-of-file comment.
    flags: u32,

    /// Monotonic id issuer for ``interp create`` without an explicit
    /// path — matches C Tcl's per-parent ``idIssuer`` on the
    /// ``Parent`` struct (``tmp/tcl9.0.3/generic/tclInterp.c``).
    /// The anonymous name ``interp<N>`` advances this counter; it
    /// stays per-parent so siblings in different parents don't
    /// collide, and so a deleted-then-recreated anonymous interp
    /// under one parent doesn't skew the issuer state in another.
    id_issuer: u32,
};

/// Root-interp singleton.  ``interp_root()`` allocates lazily and
/// adopts ``tcl_ns.ns_root()`` as its ``root_ns``.  After the first
/// call ``current_interp`` is also set.
var root_interp_addr: u32 = 0;

/// Currently-active interp.  Every ``tcl_ns.root_addr`` swap goes
/// through here too — the registry always sets the two together so
/// the ns tree and the per-interp state stay in sync.  Public so
/// cross-module callers (e.g. ``tcl_interp.eval_interp_eval``) can
/// save/restore around a swap without an extra accessor.
pub var current_interp: u32 = 0;

/// Allocate and zero-initialise a new ``Interp`` struct.
fn alloc_interp() u32 {
    const size: u32 = @sizeOf(Interp);
    const addr = alloc(size);
    const slice: [*]u8 = @ptrFromInt(addr);
    @memset(slice[0..size], 0);
    return addr;
}

/// Return the root interp, allocating it (and adopting the existing
/// ``tcl_ns`` root namespace) on first call.  Idempotent — a second
/// call returns the same address.  All the other APIs in this module
/// assume the root interp is initialised, so callers reach through
/// ``interp_current()`` which hits this on first use.
pub fn interp_root() u32 {
    if (root_interp_addr != 0) return root_interp_addr;
    root_interp_addr = alloc_interp();
    const i: *Interp = @ptrFromInt(root_interp_addr);
    i.root_ns = tcl_ns.ns_root();
    // Root interp has no parent, no simple name, no children yet.
    if (current_interp == 0) current_interp = root_interp_addr;
    return root_interp_addr;
}

/// Currently-active ``Interp`` address.  Zero means "no explicit
/// context set" — readers treat that as "root interp" and trigger
/// lazy root-interp init.
pub fn interp_current() u32 {
    if (current_interp != 0) return current_interp;
    return interp_root();
}

/// Look up a direct child of ``parent`` by simple name.  Returns 0
/// if the parent has no children table or the name doesn't match.
pub fn child_lookup(parent: u32, name_ptr: u32, name_len: u32) u32 {
    if (parent == 0) return 0;
    const p: *Interp = @ptrFromInt(parent);
    if (p.children.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (p.children.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
    }
    return 0;
}

/// Create a fresh child interp under ``parent`` with the given
/// simple name.  Allocates a new root namespace (via
/// ``tcl_ns.ns_alloc_root``), registers the child in the parent's
/// children table, and returns the new ``Interp*`` address.
///
/// Caller must have verified the name doesn't collide — a second
/// create with the same (parent, name) overwrites the old entry's
/// bucket-value slot with the new child, leaving the old child
/// unreachable from this parent (matches C Tcl's behaviour since
/// ``ChildCreate`` rejects collisions up front, which we do at the
/// callsite in ``tcl_interp.eval_interp_create``).
pub fn child_create(parent: u32, name_ptr: u32, name_len: u32, flags: u32) u32 {
    const p: *Interp = @ptrFromInt(parent);
    p.children.init(INITIAL_CAP);
    if (p.children.needs_grow()) p.children.grow();

    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = if (p.children.find(name_ptr, name_len, hash)) |b|
        b
    else
        p.children.insert_header(name_ptr, name_len, hash);

    const child = alloc_interp();
    const c: *Interp = @ptrFromInt(child);
    c.parent = parent;
    c.flags = flags;
    c.root_ns = tcl_ns.ns_alloc_root();

    if (name_len > 0) {
        const nbuf = alloc(name_len);
        memcpy(nbuf, name_ptr, name_len);
        c.name_ptr = nbuf;
        c.name_len = name_len;
    }

    write_i32(bucket + tcl_ns.OFF_HANDLE, @bitCast(child));
    return child;
}

/// Delete a child interp.  Runs the full cascade:
///
/// 1. Recursively delete every grandchild (depth-first so
///    ``INTERP_DELETED`` propagates down the tree).
/// 2. Mark the Interp as ``INTERP_DELETED`` so any lingering
///    alias redirect Command whose ``OFF_IMPORT_REF_HEAD`` stashed
///    this Interp* sees the dead bit at dispatch time and raises
///    a clean "unknown command" instead of walking a zeroed ns.
/// 3. Tombstone the parent's children-table bucket so subsequent
///    lookups ( ``child_lookup`` / ``resolve_path``) miss.
///
/// The child's namespace subtree, cmd tables, hidden-commands
/// table, and the Interp struct itself stay live in bump memory —
/// the bump allocator's "never frees" contract hasn't changed.
/// What IS guaranteed: nothing routes back into the deleted
/// interp via its old address, because every live-path check
/// (alias dispatch, name-lookup) consults ``INTERP_DELETED``
/// and/or the parent's registry bucket.
pub fn child_delete(parent: u32, name_ptr: u32, name_len: u32) bool {
    if (parent == 0) return false;
    const p: *Interp = @ptrFromInt(parent);
    if (p.children.buf == 0) return false;
    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = p.children.find(name_ptr, name_len, hash) orelse return false;
    const handle: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
    if (handle == 0) return false;

    // Step 1 — recursive teardown.  ``mark_deleted_subtree`` walks
    // the children table (and any children's children) tagging each
    // with ``INTERP_DELETED`` depth-first, then tombstones the
    // bucket slots.
    mark_deleted_subtree(handle);

    // Step 3 — tombstone the entry in the parent.
    write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
    return true;
}

/// Recursively flag ``interp`` and every descendant as
/// ``INTERP_DELETED``.  Children's child-table buckets are
/// tombstoned along the way so re-reaching them via the parent
/// chain (e.g. a stale cached handle) misses cleanly.
fn mark_deleted_subtree(interp: u32) void {
    const i: *Interp = @ptrFromInt(interp);
    i.flags |= INTERP_DELETED;
    if (i.children.buf == 0 or i.children.cap == 0) return;
    var k: u32 = 0;
    while (k < i.children.cap) : (k += 1) {
        const bucket = i.children.buf + k * BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(bucket));
        if (ep == 0) continue;
        const handle: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (handle == 0) continue;
        mark_deleted_subtree(handle);
        write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
    }
}

/// Predicate: has this Interp been torn down via ``interp delete``?
/// Dispatch paths that hold a stashed Interp* (cross-interp alias
/// redirects) call this before entering the interp.
pub fn is_deleted(interp: u32) bool {
    if (interp == 0) return true;
    const i: *Interp = @ptrFromInt(interp);
    return (i.flags & INTERP_DELETED) != 0;
}

/// Tcl_procs offsets / flag constants we reach into without
/// importing ``tcl_procs.zig`` (which imports ``tcl_ns.zig`` which
/// would create a cycle if we imported back).  Kept in sync with
/// ``tcl_procs.zig`` via the ``comptime`` assert below (using the
/// ``tcl_ns.tcl_procs_constants`` shadow that ``tcl_procs.zig``
/// already verifies against itself).
const COMMAND_SIZE: u32 = 44;
comptime {
    if (COMMAND_SIZE != tcl_ns.tcl_procs_constants.COMMAND_SIZE)
        @compileError("tcl_interp_registry.COMMAND_SIZE out of sync with tcl_ns.tcl_procs_constants.COMMAND_SIZE");
}
const OFF_CMD_NAME_PTR: u32 = 0;
const OFF_CMD_NAME_LEN: u32 = 4;
const OFF_CMD_FLAGS: u32 = 8;
const OFF_CMD_PARAMS_OBJ: u32 = 12;
const CMD_INTERP_CHILD: u32 = 0x200;

/// Allocate + populate a Command with the ``CMD_INTERP_CHILD`` flag
/// set, carrying ``child_interp`` in its ``params_obj`` slot.
/// Registered into the parent's ``cmd_table`` under the child's
/// simple name by ``eval_interp_create`` so ``myChild eval {...}``
/// resolves at the regular dispatch layer with one extra flag
/// check (same shape used by aliases).
pub fn alloc_child_command(
    parent_ns: u32,
    child_simple_ptr: u32,
    child_simple_len: u32,
    child_interp: u32,
) u32 {
    const cmd = alloc(COMMAND_SIZE);
    const slice: [*]u8 = @ptrFromInt(cmd);
    @memset(slice[0..COMMAND_SIZE], 0);

    // Stamp the FQN in the Command's name slot — same pattern
    // aliases follow.  This keeps ``info commands`` / ``namespace
    // which -command`` happy.
    const fqn = tcl_ns.ns_build_fqn(parent_ns, child_simple_ptr, child_simple_len);
    write_i32(cmd + OFF_CMD_NAME_PTR, @bitCast(fqn.ptr));
    write_i32(cmd + OFF_CMD_NAME_LEN, @bitCast(fqn.len));
    write_i32(cmd + OFF_CMD_FLAGS, @bitCast(CMD_INTERP_CHILD));
    write_i32(cmd + OFF_CMD_PARAMS_OBJ, @bitCast(child_interp));
    return cmd;
}

/// Extract the stashed ``Interp*`` from a ``CMD_INTERP_CHILD`` Command.
/// Caller should gate on the flag first.
pub fn cmd_child_interp(cmd: u32) u32 {
    return @bitCast(read_i32(cmd + OFF_CMD_PARAMS_OBJ));
}

/// Resolve an interp path (a Tcl list of simple names) relative to
/// ``base`` and return the target ``Interp*``.  Empty path returns
/// ``base`` unchanged ("current interp" shortcut).  Returns 0 if
/// any intermediate component misses.
///
/// The path list is parsed via ``tcl_obj.list_count_elements`` /
/// ``list_element_at`` so each element's simple-name span can have
/// an arbitrary origin (bump-allocated, braced, quoted — the raw
/// span is forwarded to ``child_lookup`` verbatim).  Callers that
/// need to accept qualified names (``::foo::bar``) would have to
/// pre-resolve them — which we don't: interp-path components are
/// simple names by construction (``interp0``, ``foo``, etc.).
pub fn resolve_path(base: u32, path_ptr: u32, path_len: u32) u32 {
    if (path_len == 0) return base;
    const count = obj.list_count_elements(path_ptr, path_len);
    var cur = base;
    var i: i64 = 0;
    while (i < count) : (i += 1) {
        const elem = obj.list_element_at(path_ptr, path_len, i);
        cur = child_lookup(cur, path_ptr + elem.start, elem.len);
        if (cur == 0) return 0;
    }
    return cur;
}

/// Insert (or update) a hidden-table entry in ``interp``'s hidden
/// slot.  Mirrors the former module-global ``tcl_ns.hidden_put``;
/// the move to per-interp storage lets cross-interp ``interp hide``
/// target the resolved child interp rather than a shared table.
pub fn hidden_put(interp: u32, name_ptr: u32, name_len: u32, value: u32) u32 {
    const i: *Interp = @ptrFromInt(interp);
    i.hidden_cmd_table.init(INITIAL_CAP);
    const hash = ht.fnv1a(name_ptr, name_len);
    if (i.hidden_cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        write_i32(bucket + tcl_ns.OFF_HANDLE, @bitCast(value));
        return bucket;
    }
    if (i.hidden_cmd_table.needs_grow()) i.hidden_cmd_table.grow();
    const bucket = i.hidden_cmd_table.insert_header(name_ptr, name_len, hash);
    write_i32(bucket + tcl_ns.OFF_HANDLE, @bitCast(value));
    return bucket;
}

/// Find a hidden-table entry in ``interp`` by simple name.  Returns
/// the stored Command handle or 0 if absent.
pub fn hidden_find(interp: u32, name_ptr: u32, name_len: u32) u32 {
    if (interp == 0) return 0;
    const i: *Interp = @ptrFromInt(interp);
    if (i.hidden_cmd_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (i.hidden_cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
    }
    return 0;
}

/// Tombstone a hidden-table bucket in ``interp``.  Returns true if
/// an entry was cleared.
pub fn hidden_clear(interp: u32, name_ptr: u32, name_len: u32) bool {
    if (interp == 0) return false;
    const i: *Interp = @ptrFromInt(interp);
    if (i.hidden_cmd_table.buf == 0) return false;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (i.hidden_cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        if (read_i32(bucket + tcl_ns.OFF_HANDLE) == 0) return false;
        write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
        return true;
    }
    return false;
}

/// Expose read-only access to an interp's hidden table for iterators
/// (e.g. ``interp hidden``).  The raw buffer + capacity form lets
/// callers walk buckets directly at the shared 16-byte layout.
pub fn hidden_table_buf(interp: u32) u32 {
    if (interp == 0) return 0;
    const i: *Interp = @ptrFromInt(interp);
    return i.hidden_cmd_table.buf;
}

pub fn hidden_table_cap(interp: u32) u32 {
    if (interp == 0) return 0;
    const i: *Interp = @ptrFromInt(interp);
    return i.hidden_cmd_table.cap;
}

/// Saved state captured by :func:`enter` and restored by
/// :func:`leave`.  Packed as plain ``u32`` fields so
/// save/restore is a trivial trio of loads + stores — no branch, no
/// ABI surface for the WASM caller.
pub const EnterSave = struct {
    prev_interp: u32,
    prev_root_addr: u32,
    prev_current_ns: u32,
};

/// Enter ``target`` — swap ``current_interp`` / ``tcl_ns.root_addr``
/// / ``tcl_ns.current_ns`` for the duration of a nested eval, and
/// return the prior values so :func:`leave` can restore them.
///
/// The child interp's ``current_ns`` is reset to 0 ("no explicit
/// context; use root") so top-level calls in the child script land
/// in the child's root ns, not the parent's.  Saving
/// ``tcl_ns.root_addr`` covers the case where the parent is itself
/// a child and we need to unwind correctly on return.
pub fn enter(target: u32) EnterSave {
    const save: EnterSave = .{
        .prev_interp = current_interp,
        .prev_root_addr = tcl_ns.root_addr,
        .prev_current_ns = tcl_ns.current_ns,
    };
    const t: *Interp = @ptrFromInt(target);
    current_interp = target;
    tcl_ns.root_addr = t.root_ns;
    tcl_ns.current_ns = 0;
    return save;
}

/// Inverse of :func:`enter` — restore the saved state.  Callers that
/// match ``enter`` / ``leave`` as a save/restore pair get transparent
/// nesting: ``interp eval child1 { interp eval child2 {...} }`` will
/// walk into child2 from child1 and unwind back through both.
pub fn leave(save: EnterSave) void {
    current_interp = save.prev_interp;
    tcl_ns.root_addr = save.prev_root_addr;
    tcl_ns.current_ns = save.prev_current_ns;
}
