// Namespace tree — root + child links + per-ns sub-tables.
//
// Phase P1.2 of the runtime correctness rework
// (``docs/design/runtime/namespace-tree.md``).  This module owns
// the ``Namespace`` struct and the root-ns singleton.  Nothing in
// the runtime calls into it yet — P1.3 wires
// ``tcl_ns_set``/``tcl_ns_restore`` to a real ``*Namespace``, and
// P2 flips ``proc_lookup`` to walk the tree.
//
// Storage model: every sub-table (child / cmd / var) is a
// ``hash_table.Table(NS_BUCKET_SIZE)``.  Bucket payload is a single
// u32 — a child Namespace*, a Command*, or a Var* respectively.
// All addresses are absolute u32 offsets into the bump-allocated
// linear memory managed by ``tcl_obj.zig``.
//
// Why ``extern struct``: we hand bare ``u32`` addresses across the
// runtime (matching the rest of the WASM ABI) and need a
// guaranteed layout so a ``@ptrFromInt`` cast points at the right
// fields without padding surprises.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const memcpy = obj.memcpy;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;

const ht = @import("hash_table.zig");

/// Sub-table bucket size.  All three (child / cmd / var) tables use
/// the same 12-byte header + 4-byte u32 value layout, which keeps
/// ``Table(16)`` monomorphised once and shared between them.
pub const NS_BUCKET_SIZE: u32 = 16;

/// Initial sub-table capacity.  Most namespaces have only a handful
/// of children / commands; 16 buckets keeps the load factor low
/// without wasting much linear memory.  ``Table.grow`` doubles on
/// demand.
const NS_INITIAL_CAP: u32 = 16;

/// Offset into a sub-table bucket where the value (handle) lives.
/// Equal to ``ht.HEADER_SIZE`` by construction; named for clarity
/// at call sites that read / write the handle.
pub const OFF_HANDLE: u32 = ht.HEADER_SIZE;

const ChildTable = ht.Table(NS_BUCKET_SIZE);
const CmdTable = ht.Table(NS_BUCKET_SIZE);
const VarTable = ht.Table(NS_BUCKET_SIZE);

/// Mirror of C Tcl 9's ``Namespace`` (tclInt.h:271), trimmed to the
/// fields we actually consume.  See ``docs/design/runtime/namespace-tree.md``
/// §3 for the full mapping and §4 for the deferred fields.
pub const Namespace = extern struct {
    /// Simple (unqualified) name.  ``(name_ptr, name_len)`` points
    /// into a heap-copied byte buffer owned by this struct.  Both
    /// zero for the root ns (whose simple name is the empty string).
    name_ptr: u32,
    name_len: u32,

    /// ``::``-prefixed FQN, lazily materialised from the parent
    /// chain.  Both zero until first use.  P1.2 doesn't read this
    /// — populated later when we add ``[namespace current]``.
    full_name_ptr: u32,
    full_name_len: u32,

    /// Enclosing namespace, as an absolute address.  Zero only for
    /// the root.
    parent: u32,

    /// Sub-tables.  Lazily initialised on first insert; ``buf == 0``
    /// is the "empty" marker shared with ``hash_table.Table``.
    child_table: ChildTable,
    cmd_table: CmdTable,
    var_table: VarTable,

    /// ``namespace export`` patterns.  P4.1 fills in.
    export_patterns: u32,
    export_pattern_count: u32,

    /// ``namespace path`` ordered targets.  P5.1 fills in.
    path_array: u32,
    path_len: u32,

    /// Bumped on every command add / delete in this ns; cascaded
    /// through ``path_source_head`` to invalidate dependents.  P5.3.
    cmd_ref_epoch: u32,

    /// Doubly-linked list head of ``NamespacePathEntry`` nodes
    /// whose ``target_ns`` is this ns.  P5 wires it.
    path_source_head: u32,

    /// NS_DYING / NS_DEAD / NS_TEARDOWN bits.  Stays zero in our
    /// runtime — we have no namespace deletion path (see design
    /// doc §4 "Refcounting / cleanup").
    flags: u32,
};

/// Address of the root namespace, allocated lazily on first call to
/// ``ns_root``.  Zero before then.
var root_addr: u32 = 0;

/// Allocate and zero-initialise a new ``Namespace`` at a fresh
/// linear-memory address.  Sub-tables start with ``buf == 0`` (will
/// init lazily on first insert).
fn alloc_namespace() u32 {
    const size: u32 = @sizeOf(Namespace);
    const addr = alloc(size);
    // ``alloc`` doesn't zero, so wipe the whole struct.  This is
    // important for the sub-table ``buf`` fields — a non-zero ``buf``
    // would make ``Table.init`` skip allocation and the
    // ``find``/``insert`` paths would dereference garbage.
    const slice: [*]u8 = @ptrFromInt(addr);
    @memset(slice[0..size], 0);
    return addr;
}

/// Copy ``len`` bytes of ``name`` onto the heap and stash the
/// pointer + length on the namespace's ``name_*`` fields.  The
/// source bytes may live anywhere — typically a TclObj string slab
/// that could be reclaimed; we always own a copy.
fn install_name(ns_addr: u32, name_ptr: u32, name_len: u32) void {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (name_len == 0) {
        ns.name_ptr = 0;
        ns.name_len = 0;
        return;
    }
    const buf = alloc(name_len);
    memcpy(buf, name_ptr, name_len);
    ns.name_ptr = buf;
    ns.name_len = name_len;
}

/// Return the root (global) namespace address.  Idempotent — a
/// second call returns the same address.  All other module-private
/// helpers assume the root has been initialised before they're
/// reached, so callers that touch any ``Namespace`` should call
/// ``ns_root()`` once at startup (or rely on the pattern of
/// "every ns ultimately descends from root, so creating the first
/// child triggers root creation").
pub fn ns_root() u32 {
    if (root_addr != 0) return root_addr;
    root_addr = alloc_namespace();
    // Root has empty simple name and no parent.  Sub-tables stay
    // zero until first insert.
    return root_addr;
}

/// Find a direct child of ``parent`` with simple name
/// ``(name_ptr, name_len)``.  Returns 0 if not present.
pub fn ns_lookup(parent: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(parent);
    if (ns.child_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.child_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

/// Find-or-create.  Returns the existing child if present, else
/// allocates a new ``Namespace``, links it under ``parent``, and
/// returns its address.  Sub-tables of the new child are empty.
pub fn ns_create(parent: u32, name_ptr: u32, name_len: u32) u32 {
    const existing = ns_lookup(parent, name_ptr, name_len);
    if (existing != 0) return existing;

    const ns: *Namespace = @ptrFromInt(parent);
    ns.child_table.init(NS_INITIAL_CAP);
    if (ns.child_table.needs_grow()) ns.child_table.grow();

    const hash = ht.fnv1a(name_ptr, name_len);
    const bucket = ns.child_table.insert_header(name_ptr, name_len, hash);

    const child_addr = alloc_namespace();
    install_name(child_addr, name_ptr, name_len);
    const child: *Namespace = @ptrFromInt(child_addr);
    child.parent = parent;

    write_i32(bucket + OFF_HANDLE, @bitCast(child_addr));
    return child_addr;
}

// -- WASM-exported wrappers -------------------------------------------------
//
// These give the Python-side runtime a stable ABI (i32 in / i32 out)
// that doesn't depend on Zig's slice calling convention.  All names
// are byte ranges into linear memory — same convention as
// ``tcl_globals.global_set`` etc.

pub export fn tcl_ns_root() i32 {
    return @bitCast(ns_root());
}

pub export fn tcl_ns_lookup(parent: i32, name_ptr: i32, name_len: i32) i32 {
    return @bitCast(ns_lookup(@bitCast(parent), @bitCast(name_ptr), @bitCast(name_len)));
}

pub export fn tcl_ns_create(parent: i32, name_ptr: i32, name_len: i32) i32 {
    return @bitCast(ns_create(@bitCast(parent), @bitCast(name_ptr), @bitCast(name_len)));
}
