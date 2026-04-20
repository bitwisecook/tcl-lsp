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

/// Currently-active namespace handle.  Zero means "no explicit
/// context set" — readers should treat that as root.  Compiled
/// procs flip this via ``ns_set`` / ``ns_restore`` (in
/// ``tcl_interp.zig``) before dispatching into the interpreter so
/// dynamic ``proc $name body`` lands in the enclosing ns.
///
/// Public so that ``tcl_interp.zig`` can write through it without a
/// circular dependency loop (interp imports ns; if ns imported
/// interp we'd cycle).
pub var current_ns: u32 = 0;

/// The "current namespace" with the zero-means-root convention
/// applied — every internal caller wanting a non-zero handle should
/// go through this rather than reading ``current_ns`` directly.
pub fn ns_current() u32 {
    return if (current_ns != 0) current_ns else ns_root();
}

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

/// Walk a fully-qualified namespace name like ``::tcltest`` or
/// ``::a::b::c`` and find-or-create each component starting from
/// the root.  Returns the address of the deepest namespace.
///
/// This is a P1.3 precursor to the full
/// ``TclGetNamespaceForQualName``-style resolver landing in P1.4 —
/// it handles only fully-qualified inputs (the form compiled procs
/// emit when stamping their enclosing namespace).  Non-``::``-
/// prefixed input is treated as relative to root, matching the
/// "stamped FQN" use case rather than the more general C semantics.
///
/// An empty name (or ``::`` alone) returns ``ns_root()``.
pub fn ns_create_from_fqn(name_ptr: u32, name_len: u32) u32 {
    var ns = ns_root();
    if (name_len == 0) return ns;
    const src: [*]const u8 = @ptrFromInt(name_ptr);

    // Skip leading ``::`` (zero, one, or repeated colons all read
    // as "anchor at root", matching Tcl's behaviour where two or
    // more adjacent colons are treated as a single ns separator).
    var i: u32 = 0;
    while (i < name_len and src[i] == ':') : (i += 1) {}

    // Walk components separated by one-or-more ``:``s.
    while (i < name_len) {
        // Find the end of this component (start of next ``:`` run
        // or end of string).
        var j: u32 = i;
        while (j < name_len and src[j] != ':') : (j += 1) {}
        const comp_len: u32 = j - i;
        if (comp_len > 0) {
            ns = ns_create(ns, name_ptr + i, comp_len);
        }
        // Skip the ``:`` run.
        i = j;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }
    return ns;
}

/// Materialise the namespace's fully-qualified name (``::a::b``) on
/// demand and cache it in ``full_name_*``.  Root returns ``::``.
/// Subsequent calls return the cached pointer + length.
pub fn ns_full_name(ns_addr: u32) struct { ptr: u32, len: u32 } {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (ns.full_name_ptr != 0 or ns.full_name_len != 0) {
        return .{ .ptr = ns.full_name_ptr, .len = ns.full_name_len };
    }

    if (ns.parent == 0) {
        // Root namespace: full name is the literal ``::``.
        const buf = alloc(2);
        const dst: [*]u8 = @ptrFromInt(buf);
        dst[0] = ':';
        dst[1] = ':';
        ns.full_name_ptr = buf;
        ns.full_name_len = 2;
        return .{ .ptr = buf, .len = 2 };
    }

    // ``<parent_full>::<simple>``.  Materialise parent first; if
    // parent is root, its full name is ``::`` and we'd produce
    // ``::::name`` if we naively concatenated, so collapse the case.
    const parent_full = ns_full_name(ns.parent);
    const total: u32 = blk: {
        if (parent_full.len == 2) {
            // Parent is root → ``::name``.
            break :blk 2 + ns.name_len;
        }
        // Parent is anything else → ``<parent>::<name>``.
        break :blk parent_full.len + 2 + ns.name_len;
    };
    const buf = alloc(total);
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    if (parent_full.len == 2) {
        dst[0] = ':';
        dst[1] = ':';
        off = 2;
    } else {
        const ps: [*]const u8 = @ptrFromInt(parent_full.ptr);
        for (0..parent_full.len) |k| dst[k] = ps[k];
        dst[parent_full.len] = ':';
        dst[parent_full.len + 1] = ':';
        off = parent_full.len + 2;
    }
    if (ns.name_len > 0) {
        const np: [*]const u8 = @ptrFromInt(ns.name_ptr);
        for (0..ns.name_len) |k| dst[off + k] = np[k];
    }
    ns.full_name_ptr = buf;
    ns.full_name_len = total;
    return .{ .ptr = buf, .len = total };
}

/// Result of a qualified-name resolution.  Matches the four
/// out-arguments of C Tcl's ``TclGetNamespaceForQualName`` minus
/// ``actualCxtPtrPtr`` (always derivable as ``cxt`` after the
/// global-anchoring shortcut).
///
/// ``target_ns`` and ``alt_ns`` use 0 as "not present" rather than
/// option types so the struct is plain ``extern``-friendly.  Same
/// for ``simple_*`` — zero pointer with zero length means "qualified
/// name was ``::`` alone" (or trailing ``::``), the resolution
/// landed on a namespace itself.
pub const QualifiedResult = extern struct {
    target_ns: u32,
    simple_ptr: u32,
    simple_len: u32,
    alt_ns: u32,
};

/// Resolve a (possibly qualified) name within a context namespace
/// — the find-only variant.  Mirrors ``TclGetNamespaceForQualName``
/// (``tclNamesp.c:2272``) without the create-on-miss flag.
///
/// Inputs are byte ranges into linear memory; the returned
/// ``simple_ptr`` is a pointer **into the caller's input buffer**,
/// not a heap copy.  Callers must keep ``name_ptr`` alive while
/// they consume the result.
///
/// Behaviour summary (see ``docs/design/runtime/namespace-tree.md``
/// §5.1):
///
/// * ``::``-prefixed names anchor at root; non-prefixed names anchor
///   at ``cxt``.
/// * The "alt" search path runs in parallel from root whenever the
///   primary path doesn't already start there — same use as
///   ``Tcl_FindCommand`` looking up a partially-qualified name in
///   both the current ns and the global ns.
/// * Missing intermediates set the corresponding handle to 0; the
///   walk still continues so we always report the trailing simple
///   name in ``simple_*``.
/// * ``::`` alone (or empty input) returns root with empty
///   ``simple_*`` and ``alt_ns == 0``.
pub fn ns_resolve_qualified(cxt: u32, name_ptr: u32, name_len: u32) QualifiedResult {
    var result: QualifiedResult = .{
        .target_ns = 0,
        .simple_ptr = 0,
        .simple_len = 0,
        .alt_ns = 0,
    };

    const root = ns_root();

    // 1. Anchor.  ``::``-prefixed → root; else ``cxt`` (or root if
    //    no context was given).
    var ns: u32 = if (cxt != 0) cxt else root;
    var i: u32 = 0;
    if (name_len >= 2) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        if (src[0] == ':' and src[1] == ':') {
            ns = root;
            i = 2;
            // Skip any further ``:`` (Tcl treats two-or-more
            // adjacent colons as a single separator).
            while (i < name_len and src[i] == ':') : (i += 1) {}
        }
    }

    // 2. Set up the alternate "from root" path — only meaningful
    //    when the primary path is NOT already root.
    var alt: u32 = if (ns == root) 0 else root;

    // 3. Trivial case: no remaining content → resolution landed on
    //    ``ns`` itself with an empty simple name.
    if (i >= name_len) {
        result.target_ns = ns;
        // alt stays 0 — this matches C's "altNsPtrPtr = NULL" for
        // the ``::``-only special case (tclNamesp.c:2370-2375).
        return result;
    }

    // 4. Walk components.  Each iteration consumes one
    //    ``::``-delimited component; the last component (no
    //    trailing ``::``) becomes the simple name.
    const src: [*]const u8 = @ptrFromInt(name_ptr);
    while (i < name_len) {
        // Find component end: position of the next ``::`` or end of
        // input.  We look for a single ``:`` byte first, then
        // confirm a second follows; a lone ``:`` inside a name is
        // legal in Tcl (rare — usually appears in ensembled cmd
        // names) and should NOT be treated as a separator.
        var end: u32 = i;
        var has_sep = false;
        while (end < name_len) : (end += 1) {
            if (src[end] == ':' and end + 1 < name_len and src[end + 1] == ':') {
                has_sep = true;
                break;
            }
        }
        const comp_len: u32 = end - i;

        if (!has_sep) {
            // Last component — the simple name.
            result.target_ns = ns;
            result.simple_ptr = name_ptr + i;
            result.simple_len = comp_len;
            result.alt_ns = alt;
            return result;
        }

        // Intermediate — descend in primary and alt paths.  A 0
        // handle on either side means we're already off-tree; a
        // ``find`` miss flips the side to 0 too.
        if (comp_len > 0) {
            if (ns != 0) {
                ns = child_lookup(ns, name_ptr + i, comp_len);
            }
            if (alt != 0) {
                alt = child_lookup(alt, name_ptr + i, comp_len);
            }
        }

        // Advance past the ``::`` separator and any extra colons.
        i = end + 2;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }

    // 5. Trailing ``::`` — name was ``::a::`` or similar.  ``ns``
    //    is the namespace itself; simple name is empty.
    result.target_ns = ns;
    result.alt_ns = alt;
    return result;
}

/// Same shape as ``ns_resolve_qualified`` but find-or-creates each
/// intermediate namespace along the walk.  Used by registration
/// paths (``proc_register`` dual-write, ``namespace eval``, etc.)
/// where a missing intermediate should be materialised, not
/// reported as a miss.
///
/// ``alt_ns`` is always 0 in the result — when we're creating, the
/// "search-from-root alternate" doesn't apply (we already committed
/// to a single tree location).
pub fn ns_resolve_qualified_creating(cxt: u32, name_ptr: u32, name_len: u32) QualifiedResult {
    var result: QualifiedResult = .{
        .target_ns = 0,
        .simple_ptr = 0,
        .simple_len = 0,
        .alt_ns = 0,
    };

    const root = ns_root();
    var ns: u32 = if (cxt != 0) cxt else root;
    var i: u32 = 0;
    if (name_len >= 2) {
        const src: [*]const u8 = @ptrFromInt(name_ptr);
        if (src[0] == ':' and src[1] == ':') {
            ns = root;
            i = 2;
            while (i < name_len and src[i] == ':') : (i += 1) {}
        }
    }

    if (i >= name_len) {
        result.target_ns = ns;
        return result;
    }

    const src: [*]const u8 = @ptrFromInt(name_ptr);
    while (i < name_len) {
        var end: u32 = i;
        var has_sep = false;
        while (end < name_len) : (end += 1) {
            if (src[end] == ':' and end + 1 < name_len and src[end + 1] == ':') {
                has_sep = true;
                break;
            }
        }
        const comp_len: u32 = end - i;

        if (!has_sep) {
            result.target_ns = ns;
            result.simple_ptr = name_ptr + i;
            result.simple_len = comp_len;
            return result;
        }

        if (comp_len > 0) {
            ns = ns_create(ns, name_ptr + i, comp_len);
        }

        i = end + 2;
        while (i < name_len and src[i] == ':') : (i += 1) {}
    }

    result.target_ns = ns;
    return result;
}

/// Insert (or update) a command in ``ns.cmd_table`` keyed by simple
/// name.  Returns the bucket base so the caller can later read /
/// rewrite the value.  P2.x stores the corresponding flat
/// ``proc_table`` bucket address as the value; P3+/P4 may swap to
/// a real ``Command`` struct.
///
/// The key bytes are heap-copied by ``Table.insert_header`` so the
/// caller's input buffer can be released after the call.
pub fn ns_cmd_put(ns_addr: u32, name_ptr: u32, name_len: u32, value: u32) u32 {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    ns.cmd_table.init(NS_INITIAL_CAP);
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        write_i32(bucket + OFF_HANDLE, @bitCast(value));
        return bucket;
    }
    if (ns.cmd_table.needs_grow()) ns.cmd_table.grow();
    const bucket = ns.cmd_table.insert_header(name_ptr, name_len, hash);
    write_i32(bucket + OFF_HANDLE, @bitCast(value));
    return bucket;
}

/// Find a command in ``ns.cmd_table`` by simple name.  Returns the
/// stored value (i.e. flat-bucket address while P2.x is in dual-
/// write mode) or 0 if absent.
pub fn ns_cmd_find(ns_addr: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.cmd_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

/// Lower-level child lookup: returns the child handle, or 0 if not
/// present.  Splits out so ``ns_resolve_qualified`` doesn't have to
/// recompute the hash for every step (which it doesn't anyway, but
/// the dedicated inline keeps the hot path tidy).
inline fn child_lookup(parent: u32, name_ptr: u32, name_len: u32) u32 {
    const ns: *Namespace = @ptrFromInt(parent);
    if (ns.child_table.buf == 0) return 0;
    const hash = ht.fnv1a(name_ptr, name_len);
    if (ns.child_table.find(name_ptr, name_len, hash)) |bucket| {
        return @bitCast(read_i32(bucket + OFF_HANDLE));
    }
    return 0;
}

// -- Test scaffolding -------------------------------------------------------
//
// The QualifiedResult struct is awkward to surface across a WASM
// FFI (multi-value returns aren't universally supported by hosts),
// so the test path stashes the result into module-level globals
// and exposes per-field accessors.  Production callers use the
// Zig-native ``ns_resolve_qualified`` directly.

var last_target_ns: u32 = 0;
var last_simple_ptr: u32 = 0;
var last_simple_len: u32 = 0;
var last_alt_ns: u32 = 0;

pub export fn tcl_ns_resolve_qualified(cxt: i32, name_ptr: i32, name_len: i32) i32 {
    const r = ns_resolve_qualified(@bitCast(cxt), @bitCast(name_ptr), @bitCast(name_len));
    last_target_ns = r.target_ns;
    last_simple_ptr = r.simple_ptr;
    last_simple_len = r.simple_len;
    last_alt_ns = r.alt_ns;
    return @bitCast(r.target_ns);
}

pub export fn tcl_ns_last_simple_ptr() i32 {
    return @bitCast(last_simple_ptr);
}

pub export fn tcl_ns_last_simple_len() i32 {
    return @bitCast(last_simple_len);
}

pub export fn tcl_ns_last_alt() i32 {
    return @bitCast(last_alt_ns);
}

/// Test-only: allocate ``len`` bytes via the bump allocator and
/// return the address.  Python tests use this to stage name bytes
/// into linear memory before calling resolution helpers.  Not
/// emitted by the compiler — would otherwise be a subtle DOS
/// vector if user code could call it directly.
pub export fn tcl_test_alloc(len: i32) i32 {
    return @bitCast(alloc(@bitCast(len)));
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
