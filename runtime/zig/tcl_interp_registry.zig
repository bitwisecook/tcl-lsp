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

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("hash_table.zig");
const tcl_ns = @import("tcl_ns.zig");

/// ``-safe`` marker.  Matches C Tcl's ``INTERP_SAFE`` flag semantics
/// loosely — we record the bit but don't gate file/exec/package/env
/// access because the WASM runtime doesn't expose those anyway.  See
/// ``docs/design/runtime/child-interp.md`` §2 for the deferred-gating
/// discussion.
pub const INTERP_SAFE: u32 = 0x1;

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

/// Delete a child interp: tombstone the parent's children-table
/// bucket.  Full cascade (clearing the child's command table,
/// hidden table, namespace subtree, etc.) is left to the bump
/// allocator — those byte regions stay live but unreachable from
/// the parent's registry, which matches the
/// "bump-allocator never frees" contract across the rest of the
/// runtime.  The stale Interp* address is never re-dispatched
/// because every lookup goes through the parent's children table.
pub fn child_delete(parent: u32, name_ptr: u32, name_len: u32) bool {
    if (parent == 0) return false;
    const p: *Interp = @ptrFromInt(parent);
    if (p.children.buf == 0) return false;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (p.children.find(name_ptr, name_len, hash)) |bucket| {
        if (read_i32(bucket + tcl_ns.OFF_HANDLE) == 0) return false;
        write_i32(bucket + tcl_ns.OFF_HANDLE, 0);
        return true;
    }
    return false;
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
